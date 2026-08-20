//! Ephemeral owner-conversation streaming.
//!
//! OrgIntel remains the one writer of completed messages. This module owns only
//! the live, reconnectable projection between an owner's recorded message and
//! the actor's final recorded reply: visible response text, bounded tool
//! activity, and generated-output usage. Private thought text never enters the
//! owner projection. A daemon restart may
//! lose this projection without losing company truth.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::watch;
use uuid::Uuid;

use crate::acp::{LiveSessionEvent, SessionObserver};

const MAX_ACTIVITY_ITEMS: usize = 64;
const MAX_REPLY_CHARS: usize = 40_000;
const INTENT_MARKER: &str = "<!--restless-intent:";
const DETAILS_MARKER: &str = "<!--restless-details:";
const HIDDEN_MARKERS: [&str; 2] = [DETAILS_MARKER, INTENT_MARKER];

type StreamKey = (String, String, i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationPhase {
    Queued,
    Thinking,
    Acting,
    Responding,
    Complete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationActivity {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationLiveState {
    pub stream_id: Uuid,
    pub sequence: u64,
    pub company: String,
    pub actor_id: String,
    pub trigger_message_id: i64,
    pub work_id: Option<Uuid>,
    pub phase: ConversationPhase,
    pub reply: String,
    pub generated_output_tokens: Option<u64>,
    pub activity: Vec<ConversationActivity>,
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
}

impl ConversationLiveState {
    fn queued(company: &str, actor: &str, message_id: i64, work_id: Option<Uuid>) -> Self {
        Self {
            stream_id: Uuid::new_v4(),
            sequence: 1,
            company: company.to_string(),
            actor_id: actor.to_string(),
            trigger_message_id: message_id,
            work_id,
            phase: ConversationPhase::Queued,
            reply: String::new(),
            generated_output_tokens: None,
            activity: Vec::new(),
            started_at: None,
            updated_at: Utc::now(),
            completed_message_id: None,
            error: None,
            reply_message_id: None,
            reply_pending: String::new(),
            suppress_reply_tail: false,
        }
    }

    fn interrupted(company: &str, actor: &str, message_id: i64) -> Self {
        let mut state = Self::queued(company, actor, message_id, None);
        state.fail("The prior reply was interrupted and has no live session to reconnect to.");
        state
    }

    fn touch(&mut self) {
        self.sequence = self.sequence.saturating_add(1);
        self.updated_at = Utc::now();
    }

    fn begin(&mut self) {
        self.phase = ConversationPhase::Thinking;
        self.reply.clear();
        self.generated_output_tokens = None;
        self.activity.clear();
        self.started_at = Some(Utc::now());
        self.completed_message_id = None;
        self.error = None;
        self.reply_message_id = None;
        self.reply_pending.clear();
        self.suppress_reply_tail = false;
        self.touch();
    }

    fn apply(&mut self, event: &LiveSessionEvent) {
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
                self.phase = ConversationPhase::Responding;
                if !self.suppress_reply_tail {
                    self.append_reply_delta(text);
                }
            }
            LiveSessionEvent::ThoughtDelta => {
                self.phase = ConversationPhase::Thinking;
                // Thought chunks are liveness, not owner content. Raw chain of
                // thought is intentionally absent from the projection.
            }
            LiveSessionEvent::ToolStarted { id, title, kind } => {
                self.phase = ConversationPhase::Acting;
                self.activity.push(ConversationActivity {
                    id: format!("tool-{id}"),
                    kind: "tool".into(),
                    label: title.clone(),
                    detail: kind.clone(),
                    status: "active".into(),
                });
            }
            LiveSessionEvent::ToolUpdated { id, title, status } => {
                self.phase = ConversationPhase::Acting;
                let activity_id = format!("tool-{id}");
                if let Some(existing) = self
                    .activity
                    .iter_mut()
                    .rev()
                    .find(|activity| activity.id == activity_id)
                {
                    if let Some(title) = title {
                        existing.label = title.clone();
                    }
                    existing.status = status.clone();
                }
            }
            LiveSessionEvent::GeneratedOutputTokens(tokens) => {
                self.generated_output_tokens = Some(*tokens);
            }
        }
        if self.activity.len() > MAX_ACTIVITY_ITEMS {
            self.activity
                .drain(0..self.activity.len() - MAX_ACTIVITY_ITEMS);
        }
        self.touch();
    }

    fn complete(&mut self, message_id: i64, output_tokens: Option<u64>) {
        if !self.suppress_reply_tail && !self.reply_pending.is_empty() {
            append_bounded(&mut self.reply, &self.reply_pending, MAX_REPLY_CHARS);
            self.reply_pending.clear();
        }
        self.phase = ConversationPhase::Complete;
        self.completed_message_id = Some(message_id);
        if output_tokens.is_some() {
            self.generated_output_tokens = output_tokens;
        }
        for activity in &mut self.activity {
            if activity.status == "active" {
                activity.status = "complete".into();
            }
        }
        self.touch();
    }

    fn fail(&mut self, message: &str) {
        self.phase = ConversationPhase::Failed;
        self.error = Some(bounded(message, 500));
        self.touch();
    }

    fn begin_reply_message(&mut self) {
        if !self.suppress_reply_tail && !self.reply_pending.is_empty() {
            let pending = std::mem::take(&mut self.reply_pending);
            append_bounded(&mut self.reply, &pending, MAX_REPLY_CHARS);
        }
        self.reply_pending.clear();
        self.suppress_reply_tail = false;
        if !self.reply.is_empty() && !self.reply.ends_with("\n\n") {
            append_bounded(&mut self.reply, "\n\n", MAX_REPLY_CHARS);
        }
    }

    /// Keep partial metadata markers out of the public reply. This avoids
    /// briefly flashing `<!--restless-…` when ACP splits a marker across
    /// several chunks, without delaying ordinary text.
    fn append_reply_delta(&mut self, text: &str) {
        self.reply_pending.push_str(text);
        if let Some(marker) = HIDDEN_MARKERS
            .iter()
            .filter_map(|marker| self.reply_pending.find(marker))
            .min()
        {
            append_bounded(
                &mut self.reply,
                &self.reply_pending[..marker],
                MAX_REPLY_CHARS,
            );
            while self.reply.ends_with('\n')
                || self.reply.ends_with(' ')
                || self.reply.ends_with('\t')
            {
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
            append_bounded(
                &mut self.reply,
                &self.reply_pending[..ready],
                MAX_REPLY_CHARS,
            );
            self.reply_pending.drain(..ready);
        }
    }
}

#[derive(Clone, Default)]
pub struct ConversationStreams {
    streams: Arc<Mutex<HashMap<StreamKey, watch::Sender<ConversationLiveState>>>>,
}

impl ConversationStreams {
    pub fn expect(&self, company: &str, actor: &str, message_id: i64, work_id: Option<Uuid>) {
        let key = (company.to_string(), actor.to_string(), message_id);
        if let Ok(mut streams) = self.streams.lock() {
            // The UI follows the newest owner message for an actor. Preserve a
            // genuinely live turn, but retire superseded queued/terminal
            // projections so ordinary conversation cannot grow this cache
            // without bound.
            streams.retain(
                |(stream_company, stream_actor, stream_message_id), sender| {
                    if stream_company != company
                        || stream_actor != actor
                        || *stream_message_id == message_id
                    {
                        return true;
                    }
                    matches!(
                        sender.borrow().phase,
                        ConversationPhase::Thinking
                            | ConversationPhase::Acting
                            | ConversationPhase::Responding
                    )
                },
            );
            if let Some(sender) = streams.get(&key) {
                sender.send_modify(|state| {
                    if state.work_id.is_none() {
                        state.work_id = work_id;
                        state.touch();
                    }
                });
            } else {
                let (sender, _) = watch::channel(ConversationLiveState::queued(
                    company, actor, message_id, work_id,
                ));
                streams.insert(key, sender);
            }
        }
    }

    pub fn subscribe(
        &self,
        company: &str,
        actor: &str,
        message_id: i64,
    ) -> watch::Receiver<ConversationLiveState> {
        let key = (company.to_string(), actor.to_string(), message_id);
        let streams = self.streams.lock().expect("conversation stream registry");
        if let Some(sender) = streams.get(&key) {
            return sender.subscribe();
        }

        // A subscriber is an observer, not evidence that work is queued. The
        // durable message may have outlived a daemon/ACP crash; represent that
        // absence honestly and let scheduler reconciliation start a fresh
        // projection if the unread message is still owed.
        watch::channel(ConversationLiveState::interrupted(
            company, actor, message_id,
        ))
        .1
    }

    pub fn start(&self, company: &str, actor: &str, message_ids: &[i64]) -> ConversationTurn {
        let mut senders = Vec::new();
        if let Ok(mut streams) = self.streams.lock() {
            for message_id in message_ids {
                let key = (company.to_string(), actor.to_string(), *message_id);
                let sender = streams
                    .entry(key)
                    .or_insert_with(|| {
                        watch::channel(ConversationLiveState::queued(
                            company,
                            actor,
                            *message_id,
                            None,
                        ))
                        .0
                    })
                    .clone();
                sender.send_modify(ConversationLiveState::begin);
                senders.push(sender);
            }
        }
        ConversationTurn { senders }
    }
}

#[derive(Clone)]
pub struct ConversationTurn {
    senders: Vec<watch::Sender<ConversationLiveState>>,
}

impl ConversationTurn {
    pub fn observer(&self) -> SessionObserver {
        let turn = self.clone();
        Arc::new(move |event| turn.apply(event))
    }

    pub fn apply(&self, event: LiveSessionEvent) {
        for sender in &self.senders {
            sender.send_modify(|state| state.apply(&event));
        }
    }

    pub fn complete(&self, message_id: i64, output_tokens: Option<u64>) {
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

fn append_bounded(target: &mut String, addition: &str, max_chars: usize) {
    target.push_str(addition);
    if target.chars().count() > max_chars {
        *target = target
            .chars()
            .rev()
            .take(max_chars)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
    }
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_reply_keeps_activity_collapsed_data_separate_and_finishes_with_output_usage() {
        let streams = ConversationStreams::default();
        streams.expect("company_test", "lead", 42, None);
        let receiver = streams.subscribe("company_test", "lead", 42);
        let turn = streams.start("company_test", "lead", &[42]);

        turn.apply(LiveSessionEvent::ThoughtDelta);
        turn.apply(LiveSessionEvent::ToolStarted {
            id: "call-1".into(),
            title: "Read release report".into(),
            kind: "read".into(),
        });
        turn.apply(LiveSessionEvent::ToolUpdated {
            id: "call-1".into(),
            title: None,
            status: "completed".into(),
        });
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "The evidence now ".into(),
        });
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "supports release.".into(),
        });
        turn.apply(LiveSessionEvent::GeneratedOutputTokens(137));
        turn.complete(99, Some(137));

        let state = receiver.borrow().clone();
        assert_eq!(state.phase, ConversationPhase::Complete);
        assert_eq!(state.reply, "The evidence now supports release.");
        assert_eq!(state.generated_output_tokens, Some(137));
        assert_eq!(state.completed_message_id, Some(99));
        assert_eq!(state.activity.len(), 1);
        assert_eq!(state.activity[0].kind, "tool");
        assert_eq!(state.activity[0].status, "completed");

        let reconnected = streams.subscribe("company_test", "lead", 42);
        assert_eq!(reconnected.borrow().sequence, state.sequence);
        assert_eq!(reconnected.borrow().reply, state.reply);
        assert_eq!(reconnected.borrow().generated_output_tokens, Some(137));
    }

    #[test]
    fn subscribing_does_not_invent_a_queued_turn() {
        let streams = ConversationStreams::default();
        let absent = streams.subscribe("company_test", "exec", 43);
        assert_eq!(absent.borrow().phase, ConversationPhase::Failed);
        assert!(absent
            .borrow()
            .error
            .as_deref()
            .is_some_and(|error| error.contains("no live session")));

        streams.expect("company_test", "exec", 43, None);
        let expected = streams.subscribe("company_test", "exec", 43);
        assert_eq!(expected.borrow().phase, ConversationPhase::Queued);
    }

    #[test]
    fn a_new_visible_message_appends_to_the_visible_reply() {
        let streams = ConversationStreams::default();
        streams.expect("company_test", "lead", 7, None);
        let receiver = streams.subscribe("company_test", "lead", 7);
        let turn = streams.start("company_test", "lead", &[7]);
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("round-1".into()),
            text: "I am checking that now.".into(),
        });
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("round-2".into()),
            text: "The final answer is ready.".into(),
        });

        let state = receiver.borrow().clone();
        assert_eq!(
            state.reply,
            "I am checking that now.\n\nThe final answer is ready."
        );
        assert!(state.activity.is_empty());
    }

    #[test]
    fn a_tool_call_does_not_remove_visible_reply_text() {
        let streams = ConversationStreams::default();
        streams.expect("company_test", "lead", 8, None);
        let receiver = streams.subscribe("company_test", "lead", 8);
        let turn = streams.start("company_test", "lead", &[8]);
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "I have the first finding.".into(),
        });

        turn.apply(LiveSessionEvent::ThoughtDelta);
        turn.apply(LiveSessionEvent::ToolStarted {
            id: "call-2".into(),
            title: "Read mobile report".into(),
            kind: "read".into(),
        });

        let state = receiver.borrow().clone();
        assert_eq!(state.reply, "I have the first finding.");
        assert_eq!(state.activity.len(), 1);
    }

    #[test]
    fn a_split_intent_marker_never_flashes_in_the_visible_reply() {
        let streams = ConversationStreams::default();
        streams.expect("company_test", "lead", 9, None);
        let receiver = streams.subscribe("company_test", "lead", 9);
        let turn = streams.start("company_test", "lead", &[9]);
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "Answer.\n\n<!--restless".into(),
        });
        assert_eq!(receiver.borrow().reply, "Answer.\n\n");

        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "-intent:{\"kind\":\"conversation\"}-->".into(),
        });
        assert_eq!(receiver.borrow().reply, "Answer.");
    }

    #[test]
    fn a_split_details_marker_never_flashes_in_the_visible_reply() {
        let streams = ConversationStreams::default();
        streams.expect("company_test", "lead", 10, None);
        let receiver = streams.subscribe("company_test", "lead", 10);
        let turn = streams.start("company_test", "lead", &[10]);
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "Answer.\n\n<!--restless".into(),
        });
        assert_eq!(receiver.borrow().reply, "Answer.\n\n");

        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "-details:{\"markdown\":\"commit abc\"}-->".into(),
        });
        assert_eq!(receiver.borrow().reply, "Answer.");
    }
}
