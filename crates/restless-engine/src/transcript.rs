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
    /// Up to three short answers the agent expects to `owner_need`. The
    /// cockpit offers them as drafts; the owner still sends their own words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_replies: Vec<String>,
    /// Up to four short labels for the separate facts an `owner_need`
    /// asks for ("Price paid", "Load test result"). The cockpit gives each
    /// one a field and sends the answers as ordinary text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_fields: Vec<String>,
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

pub fn split_intent_receipt(body: &str) -> (&str, Option<OwnerIntentReceipt>) {
    let Some((visible, encoded)) = body.rsplit_once(INTENT_MARKER) else {
        return (body, None);
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

/// The exact owner input a message asks for, with its visible text. Attention
/// reads this decoded receipt, never the prose.
pub fn conversation_owner_need(
    message: restless_orgintel::MessageRow,
) -> Option<(String, String)> {
    let decoded = decode_body(&message.body);
    let need = decoded.intent?.owner_need?;
    let need = need.trim();
    if need.is_empty() {
        return None;
    }
    Some((decoded.body.to_string(), need.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
