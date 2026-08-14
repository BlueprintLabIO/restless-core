//! Inbound events become authoritative effects, then OrgIntel messages (S03-T3).
//!
//! This implements `cross-layer-contract §3.1` the hard way, and the ordering
//! is the whole ticket:
//!
//! 1. **The Authority layer authors.** The world did something; that is an
//!    external fact and the Authority Plane owns it. It lands as an `effect`
//!    event with a receipt shape, exactly like an outbound send.
//! 2. **OrgIntel projects.** The reply becomes a message in the Exec's inbox so
//!    the agent can read it and the scheduler can wake on it. OrgIntel is a
//!    reader of what the world did, **never a second writer** of it.
//!
//! Getting this backwards — writing the message first and the receipt later, or
//! only writing the message — would make the company's own coordination store
//! the record of an external fact. §3.1's rule 3 forbids it, and the practical
//! cost is that reconciliation could no longer tell what actually happened from
//! what the company believes happened. Aris has already once claimed £45 of
//! revenue that receipts put at £18.
//!
//! Dedupe is by the provider's event id, because the thing being deduplicated
//! is a *redelivery of one provider event*, not a repeated request of ours.

use std::sync::Arc;

use anyhow::{Context as _, Result};

use crate::ingress::{InboundEvent, Sink};

/// The live sink: hands each verified event to a task that records and projects
/// it. The ingress never blocks on this — it enqueues and answers 202.
pub struct OrgIntelSink {
    pub daemon: Arc<crate::Daemon>,
}

impl Sink for OrgIntelSink {
    fn accept(&self, event: InboundEvent) {
        let daemon = Arc::clone(&self.daemon);
        tokio::spawn(async move {
            if let Err(error) = record_and_project(&daemon.orgintel, &event).await {
                // An event we verified but could not record is a real loss, and
                // it is logged as one. It is NOT retried here: Resend redelivers
                // on a non-2xx, and we already answered 202, so a silent retry
                // loop would be ours to get wrong. Honest gap, named.
                tracing::error!(
                    company = %event.company,
                    provider_event_id = %event.provider_event_id,
                    error = %format!("{error:#}"),
                    "verified inbound event could not be recorded"
                );
            }
        });
    }
}

/// Author the effect, then project the message. Returns whether this was new
/// (false means it was a redelivery and nothing was written twice).
pub async fn record_and_project(
    registry: &crate::OrgIntelRegistry,
    event: &InboundEvent,
) -> Result<bool> {
    let org = registry
        .get(&event.company)
        .await
        .with_context(|| format!("no OrgIntel for company {}", event.company))?;

    // Dedupe on the provider's id. A webhook redelivered after a timeout is one
    // event, and waking the company twice for one reply is exactly the
    // "double-fire" AC this sprint tests.
    if org
        .find_event_body("inbound_effect", "provider_event_id", &event.provider_event_id)
        .await?
        .is_some()
    {
        tracing::info!(
            company = %event.company,
            provider_event_id = %event.provider_event_id,
            "inbound event already recorded; ignoring redelivery"
        );
        return Ok(false);
    }

    let kind = event.body.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
    let data = event.body.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let from = first_string(&data, &["from", "sender", "email"]);
    let subject = first_string(&data, &["subject"]).unwrap_or_default();
    let text = first_string(&data, &["text", "body", "html"]).unwrap_or_default();

    // 1. Authority: the world did this.
    org.emit_event(
        "inbound_effect",
        Some("world"),
        serde_json::json!({
            "provider": "resend",
            "provider_event_id": event.provider_event_id,
            "type": kind,
            "party": from,
            "subject": subject,
            "outcome": { "status": "received" },
            "received_at": chrono::Utc::now().to_rfc3339(),
        }),
    )
    .await?;

    // 2. OrgIntel: project it so the Exec can read it and the scheduler wakes.
    //    Only for events that are actually a person talking to us — a delivery
    //    or bounce notification is an authoritative fact about our own send,
    //    already recorded above, and putting it in the Exec's inbox as if it
    //    were correspondence would be noise the agent pays tokens to read.
    if kind == "email.received" || kind == "email.inbound" {
        org.add_actor("world", "world", "The outside world").await.ok();
        org.add_actor("exec", "exec", "The Exec").await.ok();
        let body = format!(
            "REPLY from {}\nsubject: {}\n\n{}",
            from.as_deref().unwrap_or("(unknown sender)"),
            subject,
            text.chars().take(4_000).collect::<String>()
        );
        org.send_message("world", Some("exec"), &body).await?;
        tracing::info!(
            company = %event.company, from = ?from,
            "inbound reply projected into the Exec inbox; the scheduler wakes on new mail"
        );
    }
    Ok(true)
}

/// First present, non-empty string among candidate keys. Deterministic lookup,
/// never inference — providers disagree about field names and guessing which
/// one carries the sender is how a reply gets attributed to nobody.
fn first_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        // `from` is sometimes a string and sometimes {name, email}.
        match value.get(key) {
            Some(serde_json::Value::String(text)) if !text.trim().is_empty() => {
                return Some(text.trim().to_string());
            }
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::String(text)) = map.get("email") {
                    if !text.trim().is_empty() {
                        return Some(text.trim().to_string());
                    }
                }
            }
            Some(serde_json::Value::Array(items)) => {
                if let Some(serde_json::Value::String(text)) = items.first() {
                    if !text.trim().is_empty() {
                        return Some(text.trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Providers disagree about how a sender is shaped, and all three of these
    /// appear in Resend payloads depending on the event type.
    #[test]
    fn a_sender_is_found_whatever_shape_the_provider_used() {
        let string_form = serde_json::json!({ "from": "greg@example.com" });
        assert_eq!(first_string(&string_form, &["from"]).as_deref(), Some("greg@example.com"));

        let object_form = serde_json::json!({ "from": { "name": "Greg", "email": "greg@example.com" } });
        assert_eq!(first_string(&object_form, &["from"]).as_deref(), Some("greg@example.com"));

        let array_form = serde_json::json!({ "to": ["aris@blueprintlab.io"] });
        assert_eq!(first_string(&array_form, &["to"]).as_deref(), Some("aris@blueprintlab.io"));

        // Absent and blank both mean "no sender", not an empty-string sender.
        assert_eq!(first_string(&serde_json::json!({ "from": "   " }), &["from"]), None);
        assert_eq!(first_string(&serde_json::json!({}), &["from"]), None);
    }
}
