//! The owner/actor transcript body codec. A stored message body carries the
//! visible text plus trailing machine markers (attachments, intent receipt,
//! details, cockpit and Attention context). Writers append them; every
//! reader decodes them here, in one order, so no surface guesses from prose.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ATTACHMENT_BLOCK: &str = "\n\n[Restless attachments]\n";

pub const ATTACHMENT_MARKER: &str = "<!--restless-attachments:";

pub const INTENT_MARKER: &str = "<!--restless-intent:";

pub const DETAILS_MARKER: &str = "<!--restless-details:";

pub const CONTEXT_BLOCK: &str = "\n\n[Owner cockpit context]\n";

pub const CONTEXT_MARKER: &str = "\n\n<!--restless-context:";

pub const ATTENTION_CONTEXT_BLOCK: &str = "\n\n[Restless Attention context — system supplied]\n";

pub const ATTENTION_CONTEXT_MARKER: &str = "\n\n<!--restless-attention-context:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct OwnerAttachment {
    pub upload_id: Uuid,
    pub name: String,
    pub media_type: String,
    pub size_bytes: usize,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum OwnerIntentKind {
    Conversation,
    WorkFeedback,
    Direction,
    Authority,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct OwnerIntentReceipt {
    pub kind: OwnerIntentKind,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_need: Option<String>,
    /// When this is commentary on an existing prepared decision, that handoff owns the only action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_need_handoff: Option<String>,
    /// A finished result the owner should look at, named in a few words ("The redesigned card
    /// site"). It reaches the owner's Inbox, not only the chat (21 October).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_review: Option<String>,
    /// Up to three short answers the agent expects to `owner_need`. The
    /// cockpit offers them as drafts; the owner still sends their own words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_replies: Vec<String>,
    /// Up to four short labels for the separate facts an `owner_need`
    /// asks for ("Price paid", "Load test result"). The cockpit gives each
    /// one a field and sends the answers as ordinary text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_fields: Vec<String>,
    /// Exec's read-back at the end of the first conversation with a new owner: how it understands
    /// the company, line by line ("What it is", "Why it matters to you"…). The welcome page shows
    /// it for the owner to confirm; confirming saves it as the charter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readback: Vec<ReadbackLine>,
    /// The company name Exec proposes with its read-back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadbackLine {
    pub label: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct OwnerMessageDetails {
    pub markdown: String,
}

pub fn split_attention_context(body: &str) -> (&str, Option<String>) {
    let Some((visible_with_context, encoded)) = body.rsplit_once(ATTENTION_CONTEXT_MARKER) else {
        return (body, None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, None);
    };
    let Ok(marker) = serde_json::from_str::<serde_json::Value>(encoded) else {
        return (body, None);
    };
    let Some(original_bytes) = marker
        .get("original_bytes")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
    else {
        return (body, None);
    };
    let Some(item_id) = marker
        .get("item_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return (body, None);
    };
    if original_bytes > visible_with_context.len()
        || !visible_with_context.is_char_boundary(original_bytes)
        || !visible_with_context[original_bytes..].starts_with(ATTENTION_CONTEXT_BLOCK)
    {
        return (body, None);
    }
    (
        &visible_with_context[..original_bytes],
        Some(item_id.to_string()),
    )
}

pub fn split_attachment_block(body: &str) -> (&str, Vec<OwnerAttachment>) {
    let Some((visible, block)) = body.rsplit_once(ATTACHMENT_BLOCK) else {
        return (body, Vec::new());
    };
    let Some(marker) = block.rfind(ATTACHMENT_MARKER) else {
        return (body, Vec::new());
    };
    let encoded = &block[marker + ATTACHMENT_MARKER.len()..];
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, Vec::new());
    };
    match serde_json::from_str(encoded) {
        Ok(attachments) => (visible, attachments),
        Err(_) => (body, Vec::new()),
    }
}

/// A reply whose model wrote its intent as prose, `[Intent: acknowledge and revise]`, instead of
/// the marker: the line is bookkeeping, never something the owner should read (20 October).
fn without_prose_intent(body: &str) -> &str {
    let trimmed = body.trim_end();
    let (before, last) = trimmed.rsplit_once('\n').unwrap_or(("", trimmed));
    let line = last.trim();
    let is_intent = line.starts_with('[')
        && line.ends_with(']')
        && line[1..]
            .trim_start()
            .get(..7)
            .is_some_and(|word| word.eq_ignore_ascii_case("intent:"));
    if is_intent { before.trim_end() } else { body }
}

pub fn split_intent_receipt(body: &str) -> (&str, Option<OwnerIntentReceipt>) {
    let Some((visible, encoded)) = body.rsplit_once(INTENT_MARKER) else {
        return (without_prose_intent(body), None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, None);
    };
    match serde_json::from_str::<OwnerIntentReceipt>(encoded) {
        Ok(mut receipt)
            if !receipt.summary.trim().is_empty() && receipt.summary.chars().count() <= 300 =>
        {
            // Suggested answers only make sense beside a question, and stay short.
            receipt.owner_replies = if receipt.owner_need.is_some() {
                receipt
                    .owner_replies
                    .iter()
                    .map(|reply| reply.trim().to_string())
                    .filter(|reply| !reply.is_empty() && reply.chars().count() <= 80)
                    .take(3)
                    .collect()
            } else {
                Vec::new()
            };
            // A read-back is a few short lines; anything else is dropped rather than shown raw.
            receipt.readback = receipt
                .readback
                .iter()
                .map(|line| ReadbackLine {
                    label: line.label.trim().to_string(),
                    text: line.text.trim().to_string(),
                })
                .filter(|line| {
                    !line.label.is_empty()
                        && line.label.chars().count() <= 40
                        && !line.text.is_empty()
                        && line.text.chars().count() <= 400
                })
                .take(8)
                .collect();
            receipt.proposed_name = receipt
                .proposed_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty() && name.chars().count() <= 80)
                .map(str::to_string);
            // A name is proposed with a read-back, never alone: without one there is nothing for
            // the owner to confirm, and setup would stall on an empty read-back (18 October).
            if receipt.readback.is_empty() {
                receipt.proposed_name = None;
            }
            receipt.owner_fields = if receipt.owner_need.is_some() {
                receipt
                    .owner_fields
                    .iter()
                    .map(|field| field.trim().to_string())
                    .filter(|field| !field.is_empty() && field.chars().count() <= 40)
                    .take(4)
                    .collect()
            } else {
                Vec::new()
            };
            (visible.trim_end(), Some(receipt))
        }
        _ => (visible.trim_end(), None),
    }
}

pub fn split_message_details(body: &str) -> (&str, Option<String>) {
    let Some((visible, encoded)) = body.rsplit_once(DETAILS_MARKER) else {
        return (body, None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        // Metadata syntax is never owner-facing, even when a provider emits a
        // malformed optional block.
        return (visible.trim_end(), None);
    };
    let details = serde_json::from_str::<OwnerMessageDetails>(encoded)
        .ok()
        .map(|details| details.markdown.trim().to_string())
        .filter(|markdown| !markdown.is_empty() && markdown.chars().count() <= 20_000);
    (visible.trim_end(), details)
}

pub fn split_context_marker(body: &str) -> (&str, Option<String>) {
    let Some((visible, encoded)) = body.rsplit_once(CONTEXT_MARKER) else {
        return (body, None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, None);
    };
    let path = serde_json::from_str::<serde_json::Value>(encoded)
        .ok()
        .and_then(|value| {
            value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    match path {
        Some(path) => match visible.rsplit_once(CONTEXT_BLOCK) {
            Some((body, rendered)) if rendered == path => (body, Some(path)),
            _ => (visible, Some(path)),
        },
        None => (body, None),
    }
}

/// A stored body with its markers decoded, in the one order writers append them.
pub struct DecodedBody<'a> {
    pub body: &'a str,
    pub intent: Option<OwnerIntentReceipt>,
    pub details: Option<String>,
    pub attachments: Vec<OwnerAttachment>,
    pub context_path: Option<String>,
}

pub fn decode_body(raw: &str) -> DecodedBody<'_> {
    let (body, _) = split_attention_context(raw);
    let (body, intent) = split_intent_receipt(body);
    let (body, details) = split_message_details(body);
    let (body, attachments) = split_attachment_block(body);
    let (body, context_path) = split_context_marker(body);
    DecodedBody { body, intent, details, attachments, context_path }
}

/// What a message asks the owner for, from its decoded receipt.
pub struct OwnerNeed {
    /// The message's visible text.
    pub body: String,
    pub need: String,
    pub handoff: Option<String>,
    /// The short answers the agent offers, one tap each in the card.
    pub replies: Vec<String>,
    /// The separate facts it asks for, a field each in the card.
    pub fields: Vec<String>,
}

/// A finished result a message asks the owner to look at, with its visible text, when the message
/// asks for nothing else.
pub fn conversation_owner_review(message: &restless_orgintel::MessageRow) -> Option<(String, String)> {
    let decoded = decode_body(&message.body);
    let intent = decoded.intent?;
    if intent.owner_need.as_deref().is_some_and(|need| !need.trim().is_empty()) {
        return None;
    }
    let review = intent.owner_review?;
    let review = review.trim();
    if review.is_empty() || review.chars().count() > 120 {
        return None;
    }
    Some((decoded.body.to_string(), review.to_owned()))
}

/// The exact owner input a message asks for, with its visible text. Attention
/// reads this decoded receipt, never the prose.
pub fn conversation_owner_need(message: restless_orgintel::MessageRow) -> Option<OwnerNeed> {
    let decoded = decode_body(&message.body);
    let intent = decoded.intent?;
    let need = intent.owner_need?;
    let need = need.trim();
    if need.is_empty() {
        return None;
    }
    Some(OwnerNeed {
        body: decoded.body.to_string(),
        need: need.to_owned(),
        handoff: intent.owner_need_handoff,
        replies: intent.owner_replies,
        fields: intent.owner_fields,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_finished_result_for_the_owner_is_read_from_the_intent() {
        let body = "Done.\n\n<!--restless-intent:{\"kind\":\"conversation\",\"summary\":\"Built\",\"ownerReview\":\" The card site \"}-->";
        let (_, receipt) = super::split_intent_receipt(body);
        assert_eq!(receipt.unwrap().owner_review.as_deref(), Some(" The card site "));
    }

    #[test]
    fn an_intent_written_as_prose_is_not_shown() {
        let (visible, receipt) = super::split_intent_receipt(
            "Absolutely, I'll make it shorter.\n\n[Intent: acknowledge and revise]",
        );
        assert_eq!(visible, "Absolutely, I'll make it shorter.");
        assert!(receipt.is_none());
        let plain = "See [the brief] for details.";
        assert_eq!(super::split_intent_receipt(plain).0, plain);
    }

    use super::*;

    #[test]
    fn a_proposed_name_needs_a_read_back_beside_it() {
        let alone = concat!(
            "Does this sound right?",
            "\n\n<!--restless-intent:{\"kind\":\"direction\",\"summary\":\"Setting up.\",",
            "\"ownerNeed\":\"Does this sound right?\",\"proposedName\":\"Frankensurf\"}-->"
        );
        assert_eq!(split_intent_receipt(alone).1.unwrap().proposed_name, None);
        let with_readback = concat!(
            "Here is what I heard.",
            "\n\n<!--restless-intent:{\"kind\":\"direction\",\"summary\":\"Setting up.\",",
            "\"proposedName\":\"Frankensurf\",\"readback\":[{\"label\":\"What it is\",\"text\":\"A web layer.\"}]}-->"
        );
        assert_eq!(
            split_intent_receipt(with_readback).1.unwrap().proposed_name.as_deref(),
            Some("Frankensurf")
        );
    }

    #[test]
    fn only_a_valid_exec_intent_receipt_is_promoted_to_ui_metadata() {
        let body = concat!(
            "I will treat this as durable direction.",
            "\n\n<!--restless-intent:{\"kind\":\"direction\",",
            "\"summary\":\"Prioritise tutor interviews before outreach.\"}-->"
        );
        let (visible, receipt) = split_intent_receipt(body);
        assert_eq!(visible, "I will treat this as durable direction.");
        assert!(matches!(
            receipt.map(|receipt| receipt.kind),
            Some(OwnerIntentKind::Direction)
        ));

        let at_a_glance = concat!(
            "The four drafts are ready.",
            "\n\n<!--restless-intent:{\"kind\":\"conversation\",",
            "\"summary\":\"Campaign preparation result.\",",
            "\"outcome\":\"Four reviewed drafts are ready.\",",
            "\"nextStep\":\"The lead waits for the campaign decision.\",",
            "\"ownerNeed\":\"Approve, change or decline the campaign.\"}-->"
        );
        let (_, receipt) = split_intent_receipt(at_a_glance);
        let receipt = receipt.expect("optional reader fields should parse");
        assert_eq!(
            receipt.outcome.as_deref(),
            Some("Four reviewed drafts are ready.")
        );
        assert_eq!(
            receipt.owner_need.as_deref(),
            Some("Approve, change or decline the campaign.")
        );

        let suggested = concat!(
            "Ready?",
            "\n\n<!--restless-intent:{\"kind\":\"conversation\",\"summary\":\"Asks to proceed.\",",
            "\"ownerNeed\":\"Proceed with the listing?\",",
            "\"ownerReplies\":[\"Yes, list it\",\" \",\"Not yet\",\"Hold\",\"Ask me tomorrow\"]}-->"
        );
        let receipt = split_intent_receipt(suggested).1.expect("replies parse");
        assert_eq!(receipt.owner_replies, vec!["Yes, list it", "Not yet", "Hold"]);
        let fielded = concat!(
            "Need numbers.",
            "\n\n<!--restless-intent:{\"kind\":\"conversation\",\"summary\":\"Asks for numbers.\",",
            "\"ownerNeed\":\"What did the tower cost and did it pass the load test?\",",
            "\"ownerFields\":[\"Price paid\",\"\",\"Load test result\",\"CPU\",\"GPU\",\"RAM\"]}-->"
        );
        let receipt = split_intent_receipt(fielded).1.expect("fields parse");
        assert_eq!(
            receipt.owner_fields,
            vec!["Price paid", "Load test result", "CPU", "GPU"]
        );
        let without_question = concat!(
            "Done.",
            "\n\n<!--restless-intent:{\"kind\":\"conversation\",\"summary\":\"Done.\",",
            "\"ownerReplies\":[\"Thanks\"],\"ownerFields\":[\"Price\"]}-->"
        );
        let receipt = split_intent_receipt(without_question).1.expect("parses");
        assert!(receipt.owner_replies.is_empty());
        assert!(receipt.owner_fields.is_empty());

        let malformed = "Reply\n\n<!--restless-intent:{\"kind\":\"whatever\",\"summary\":\"x\"}-->";
        assert_eq!(split_intent_receipt(malformed).0, "Reply");
        assert!(split_intent_receipt(malformed).1.is_none());
    }

    #[test]
    fn optional_work_details_are_separate_and_malformed_metadata_stays_hidden() {
        let body = concat!(
            "The release is ready.",
            "\n\n<!--restless-details:{\"markdown\":\"- Commit `abc123`\\n- Build passed\"}-->"
        );
        let (visible, details) = split_message_details(body);
        assert_eq!(visible, "The release is ready.");
        assert_eq!(
            details.as_deref(),
            Some("- Commit `abc123`\n- Build passed")
        );

        let malformed = "Answer.\n\n<!--restless-details:not-json-->";
        assert_eq!(split_message_details(malformed), ("Answer.", None));
    }
}
