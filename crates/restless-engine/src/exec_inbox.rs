//! Exec manages the owner's Inbox from their chat (owner decision, 3 November 2026).
//!
//! `restless attention accept|decline|answer|snooze|dismiss|move|add` lands here. Exec names the
//! item and cites the owner's message that asked for it; this module finds the item in the same
//! projection the owner's Inbox shows, checks that the verb has a decision path for that kind of
//! item (and, for accepting a prepared outcome, that the outcome still opens), and saves the action
//! with its five-second Undo window. The owner gateway applies it when the window ends, through the
//! item's own decision path.
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::attention::AttentionItem;
use crate::daemon::Daemon;
use crate::runtime::CompanyConfig;
use restless_orgintel::{ExecInboxVerb, NewExecInboxAction};

/// The Inbox action Exec asks for (`attention_action` on the wire).
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttentionAction {
    pub verb: String,
    /// The item's id as the Inbox names it, or a handoff's id.
    #[serde(default)]
    pub item: Option<String>,
    /// The owner's reason, answer or note, in their words.
    #[serde(default)]
    pub note: Option<String>,
    /// When a snooze ends (RFC 3339).
    #[serde(default)]
    pub until: Option<String>,
    /// Where a move puts the item: `top`, `bottom` or a place counted from 1.
    #[serde(default)]
    pub to: Option<String>,
    /// A new item's title.
    #[serde(default)]
    pub title: Option<String>,
}

/// Kinds whose answer is an Authority approval the owner gives with Approve or Decline.
pub const APPROVAL_KINDS: &[&str] = &[
    "approval_required",
    "email_mandate_proposal",
    "payee_proposal",
    "account_notice",
    "account_request",
    "publication_authorization",
];

/// The id an item Exec added has in the Inbox.
pub fn added_item_id(action: uuid::Uuid) -> String {
    format!("orgintel:exec_item:{action}")
}

fn outcome_review(item: &AttentionItem) -> bool {
    item.source.kind == "owner_handoff" && item.category == "review"
}

/// Whether `verb` has a decision path for this item, and if not, what Exec should do instead.
/// Snooze, move and dismiss apply to everything in the Inbox; deciding needs a decision.
fn applies(item: &AttentionItem, verb: ExecInboxVerb, app_request: bool) -> Result<(), String> {
    let kind = item.source.kind.as_str();
    let decision = |verb: &str| -> Result<(), String> {
        match kind {
            "owner_handoff" if app_request && verb == "accept" => Err(
                "an app request is answered by adding the app in Apps; tell the owner it is one tap in its card, or decline it"
                    .into(),
            ),
            "owner_handoff" => Ok(()),
            kind if APPROVAL_KINDS.contains(&kind) => Ok(()),
            "skill_candidate" | "identity_proposal" | "direction_request" => Ok(()),
            "conversation_owner_review" if verb == "accept" => Ok(()),
            "setting_proposal" if verb == "decline" => Ok(()),
            "setting_proposal" => Err(
                "a setting is applied in its card, where the owner can adjust the value first; tell them it is one tap there"
                    .into(),
            ),
            "charter_proposal" if verb == "decline" => Ok(()),
            "charter_proposal" => Err(
                "publish the charter the owner agreed with `restless charter apply --requester owner --file <draft> --source-message <id>`"
                    .into(),
            ),
            "identity_drift" => Err("this asks the owner to choose between versions in its card".into()),
            _ => Err(format!(
                "this item has nothing to {verb}; snooze, move or dismiss it instead"
            )),
        }
    };
    match verb {
        ExecInboxVerb::Snooze | ExecInboxVerb::Move | ExecInboxVerb::Dismiss => Ok(()),
        ExecInboxVerb::Add => Err("add raises a new item; it names no existing one".into()),
        ExecInboxVerb::Accept => decision("accept"),
        ExecInboxVerb::Decline => {
            if kind == "conversation_owner_review" {
                return Err("a finished result is not declined; dismiss it, or ask for changes in a message to its lead".into());
            }
            decision("decline")
        }
        ExecInboxVerb::Answer => match kind {
            "owner_handoff" if outcome_review(item) => {
                Err("an outcome review is accepted, or declined with what should change".into())
            }
            "owner_handoff" => Ok(()),
            _ => Err(
                "only a question waiting on the owner takes an answer; accept or decline this one"
                    .into(),
            ),
        },
    }
}

/// What the decision path needs to find this item again when the window ends.
fn target_of(item: &AttentionItem) -> serde_json::Value {
    serde_json::json!({
        "kind": item.source.kind,
        "reference": item.source.reference,
        "party": item.source.party,
        "call_key": item.source.call_key,
        "account": item.account_use.as_ref().and_then(|value| value["account"]["id"].as_str()),
        "category": item.category,
        "responsible": item.responsible_actor.as_ref().map(|actor| actor.id.clone()),
        "outcome_review": outcome_review(item),
    })
}

fn position(raw: &str) -> Result<i32, String> {
    match raw.trim() {
        "top" | "first" => Ok(1),
        "bottom" | "last" => Ok(0),
        other => other
            .parse::<i32>()
            .ok()
            .filter(|place| (1..=500).contains(place))
            .ok_or_else(|| "--to takes top, bottom or a place in the list counted from 1".into()),
    }
}

fn snooze_until(raw: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let until = DateTime::parse_from_rfc3339(raw.trim())
        .map_err(|_| "--until takes a time such as 2026-11-04T09:00:00+11:00".to_string())?
        .with_timezone(&Utc);
    if until <= now {
        return Err("a snooze ends in the future".into());
    }
    if until > now + chrono::Duration::days(90) {
        return Err("a snooze ends within 90 days".into());
    }
    Ok(until)
}

/// Save one Exec action, or say plainly why it cannot be taken. Only Exec acts, and only on the
/// owner's own message to it.
pub async fn queue(
    daemon: &Daemon,
    company: &str,
    as_actor: Option<&str>,
    source_message: Option<i64>,
    action: &AttentionAction,
) -> Result<serde_json::Value, String> {
    if as_actor != Some("exec") {
        return Err("only Exec changes the owner's Inbox from their chat".into());
    }
    let verb = ExecInboxVerb::parse(action.verb.trim()).ok_or_else(|| {
        "the action is accept, decline, answer, snooze, dismiss, move or add".to_string()
    })?;
    let Some(source_message) = source_message else {
        return Err(
            "cite the owner's message that asked for this with --source-message <id>; without their words, leave the Inbox as it is"
                .into(),
        );
    };
    let note = action.note.as_deref().unwrap_or_default().trim();
    let config =
        CompanyConfig::load(&daemon.root, company).map_err(|error| format!("{error:#}"))?;
    let org = daemon
        .orgintel
        .get(company)
        .await
        .map_err(|error| format!("{error:#}"))?;
    let id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let (item_id, title, target) = if verb == ExecInboxVerb::Add {
        let title = action
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .ok_or("a new item needs --title")?;
        if title.chars().count() > 200 {
            return Err("keep the title under 200 characters".into());
        }
        (
            added_item_id(id),
            title.to_string(),
            serde_json::json!({"kind": "exec_item"}),
        )
    } else {
        let wanted = action
            .item
            .as_deref()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or("name the item with --item <id> (each line of the owner's Inbox shows it)")?;
        let view = crate::attention::project(&config, &daemon.authority, Some(&org))
            .await
            .map_err(|error| format!("the owner's Inbox could not be read: {error:#}"))?;
        let item = view
            .items
            .iter()
            .find(|item| {
                item.id == wanted
                    || (item.source.kind == "owner_handoff" && item.source.reference == wanted)
            })
            .ok_or_else(|| {
                format!(
                    "{wanted} is not in the owner's Inbox now; read it again with `restless attention --summary`"
                )
            })?;
        let app_request = item.source.kind == "owner_handoff"
            && org
                .app_requests()
                .await
                .map_err(|error| format!("{error:#}"))?
                .iter()
                .any(|request| request.handoff_id.to_string() == item.source.reference);
        applies(item, verb, app_request)?;
        if verb == ExecInboxVerb::Decline && outcome_review(item) && note.is_empty() {
            return Err(
                "say what the owner wants changed with --note; it goes back to the author".into(),
            );
        }
        // A4 on Exec's path: a go-ahead accepts only a prepared outcome that still opens. The
        // gateway checks again when the Undo window ends.
        if verb == ExecInboxVerb::Accept && outcome_review(item) {
            let handoff = uuid::Uuid::parse_str(&item.source.reference)
                .map_err(|_| "this review names no handoff".to_string())?;
            if let Some((_, reason)) = crate::attention::unavailable_review_outcome(
                &config,
                &daemon.authority,
                &org,
                handoff,
                "owner",
            )
            .await
            .map_err(|error| format!("{error:#}"))?
            {
                return Err(reason);
            }
        }
        (item.id.clone(), item.title.clone(), target_of(item))
    };
    let until = match verb {
        ExecInboxVerb::Snooze => Some(snooze_until(
            action.until.as_deref().ok_or("a snooze needs --until")?,
            now,
        )?),
        _ => None,
    };
    let place = match verb {
        ExecInboxVerb::Move => Some(position(action.to.as_deref().ok_or("a move needs --to")?)?),
        _ => None,
    };
    if matches!(verb, ExecInboxVerb::Answer | ExecInboxVerb::Add) && note.is_empty() {
        return Err("say what the owner said with --note".into());
    }
    let saved = org
        .queue_exec_inbox_action(&NewExecInboxAction {
            id,
            item_id: &item_id,
            item_title: &title,
            verb,
            note,
            target,
            snooze_until: until,
            position: place,
            source_message_id: source_message,
        })
        .await
        .map_err(|error| error.plain_message())?;
    Ok(serde_json::json!({
        "id": saved.id,
        "item": saved.item_id,
        "action": saved.verb,
        "takes_effect_at": saved.undo_until,
        "shown": "The owner's Inbox shows this beside the item with Undo; it takes effect when that window ends. Tell the owner in one line what you did.",
    }))
}

/// Exec's own recent Inbox actions, one line each, for its next turn: what it did, on whose word,
/// and how it ended (applied, undone by the owner, or refused and why).
pub async fn recent_lines(org: &restless_orgintel::OrgIntel) -> Vec<String> {
    let Ok(actions) = org.recent_exec_inbox_actions(12).await else {
        return Vec::new();
    };
    actions
        .into_iter()
        .filter(|action| action.created_at > Utc::now() - chrono::Duration::days(2))
        .map(|action| {
            let outcome = match action.state.as_str() {
                "pending" => "waiting out its Undo window".to_string(),
                "applied" => "done".to_string(),
                "canceled" => {
                    "the owner undid it; do not redo it unless they ask again".to_string()
                }
                _ => format!(
                    "refused: {}",
                    action.problem.as_deref().unwrap_or("no reason recorded")
                ),
            };
            format!(
                "{} \"{}\" from owner message {} — {outcome}",
                action.verb, action.item_title, action.source_message_id
            )
        })
        .collect()
}

/// An item Exec added from the owner's chat, as the Inbox shows it. It stays until the owner (or
/// Exec, on their word) sets it aside, and a set-aside one can be brought back like any other.
pub fn added_item(action: &restless_orgintel::ExecInboxAction) -> AttentionItem {
    use crate::attention::{AttentionActorRef, AttentionSource};
    AttentionItem {
        id: action.item_id.clone(),
        work_id: None,
        source: AttentionSource {
            plane: "orgintel",
            kind: "exec_item".into(),
            reference: action.id.to_string(),
            party: None,
            call_key: None,
        },
        category: "conversation".into(),
        title: action.item_title.clone(),
        what_happened: action.note.clone(),
        why_it_matters: "You asked Exec to keep this in your Inbox.".into(),
        recommendation: String::new(),
        requested_action: action.note.clone(),
        if_no_action: "It stays here until you skip it.".into(),
        uncertainty: None,
        deadline: None,
        brief_status: "source-authored",
        brief_author: None,
        briefed_at: None,
        evidence: Vec::new(),
        review_sources: Vec::new(),
        responsible_actor: Some(AttentionActorRef {
            id: "exec".into(),
            display: "Exec".into(),
            role: "exec".into(),
        }),
        runtime_attach: None,
        review_target: None,
        review_version: None,
        native_document: None,
        direction_request: None,
        charter_proposal: None,
        account_use: None,
        account_request: None,
        request_fields: None,
        setting_proposal: None,
        owner_replies: Vec::new(),
        actions: Vec::new(),
        can_continue: true,
        preparing: false,
        created_at: action.completed_at.unwrap_or(action.created_at),
    }
}

/// What the owner's Inbox shows beside each item Exec changed, and the place Exec gave items it
/// moved. Read with the Inbox, so a note and its order arrive with the rows they belong to.
#[derive(Debug, Default)]
pub struct ExecNotes {
    pub notes: std::collections::HashMap<String, serde_json::Value>,
    pub positions: std::collections::HashMap<String, i32>,
}

/// The note for each item: an action still in its Undo window, one that was refused today, or a
/// snooze or move that still holds. An undone action leaves no note.
pub async fn notes(org: &restless_orgintel::OrgIntel) -> Result<ExecNotes> {
    let now = Utc::now();
    let mut out = ExecNotes::default();
    for action in org.recent_exec_inbox_actions(200).await? {
        if out.notes.contains_key(&action.item_id) {
            continue;
        }
        let shown = match action.state.as_str() {
            "pending" => true,
            "failed" => action
                .completed_at
                .is_some_and(|at| now - at < chrono::Duration::hours(24)),
            "applied" => match action.verb() {
                Some(ExecInboxVerb::Snooze) => action.snooze_until.is_some_and(|until| until > now),
                Some(ExecInboxVerb::Move) => true,
                _ => false,
            },
            _ => false,
        };
        if !shown {
            continue;
        }
        out.notes.insert(
            action.item_id.clone(),
            serde_json::json!({
                "id": action.id,
                "verb": action.verb,
                "state": action.state,
                "note": action.note,
                "owner_words": action.owner_words,
                "source_message_id": action.source_message_id,
                "undo_until": action.undo_until,
                "problem": action.problem,
                "snooze_until": action.snooze_until,
                "position": action.position,
                "at": action.created_at,
            }),
        );
    }
    for (item, position, _) in org.exec_inbox_positions().await? {
        out.positions.insert(item, position);
    }
    Ok(out)
}
