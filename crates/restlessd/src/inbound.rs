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

/// The live sink: records Authority before the ingress answers 202, then
/// projects to OrgIntel best-effort.
pub struct AuthoritySink {
    pub daemon: Arc<crate::Daemon>,
}

impl Sink for AuthoritySink {
    fn accept(
        &self,
        event: InboundEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            if record_authority(&self.daemon.authority, &event).await? {
                let daemon = Arc::clone(&self.daemon);
                tokio::spawn(async move {
                    if let Err(error) = project_to_orgintel(&daemon.orgintel, &event).await {
                        tracing::warn!(
                            company = %event.company,
                            provider_event_id = %event.provider_event_id,
                            "inbound effect is durable but OrgIntel projection failed: {error:#}"
                        );
                    }
                });
            }
            Ok(())
        })
    }
}

/// Author the effect. Returns whether this was new (false means it was a
/// redelivery and nothing was written twice).
pub async fn record_authority(
    authority: &crate::authority::AuthorityStore,
    event: &InboundEvent,
) -> Result<bool> {
    let kind = event
        .body
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    let data = event
        .body
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let from = first_string(&data, &["from", "sender", "email"]);
    let subject = first_string(&data, &["subject"]).unwrap_or_default();

    // 1. Authority: the world did this.
    let inserted = authority
        .emit_inbound_once(
            &event.company,
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
    if !inserted {
        tracing::info!(
            company = %event.company,
            provider_event_id = %event.provider_event_id,
            "inbound event already recorded; ignoring redelivery"
        );
        return Ok(false);
    }

    Ok(true)
}

/// Best-effort OrgIntel projection after Authority has custody. Only events
/// that are actually a person talking to us become correspondence; delivery
/// or bounce observations stay receipts and do not consume agent attention.
async fn project_to_orgintel(
    registry: &crate::OrgIntelRegistry,
    event: &InboundEvent,
) -> Result<()> {
    let kind = event
        .body
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    if kind == "email.received" || kind == "email.inbound" {
        let data = event
            .body
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let from = first_string(&data, &["from", "sender", "email"]);
        let subject = first_string(&data, &["subject"]).unwrap_or_default();
        let text = first_string(&data, &["text", "body", "html"]).unwrap_or_default();
        let org = registry.get(&event.company).await.with_context(|| {
            format!(
                "no OrgIntel projection target for company {}",
                event.company
            )
        })?;
        org.ensure_actor("world", "system", "external-sender", "The outside world")
            .await
            .ok();
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .ok();
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
    Ok(())
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
        assert_eq!(
            first_string(&string_form, &["from"]).as_deref(),
            Some("greg@example.com")
        );

        let object_form =
            serde_json::json!({ "from": { "name": "Greg", "email": "greg@example.com" } });
        assert_eq!(
            first_string(&object_form, &["from"]).as_deref(),
            Some("greg@example.com")
        );

        let array_form = serde_json::json!({ "to": ["aris@blueprintlab.io"] });
        assert_eq!(
            first_string(&array_form, &["to"]).as_deref(),
            Some("aris@blueprintlab.io")
        );

        // Absent and blank both mean "no sender", not an empty-string sender.
        assert_eq!(
            first_string(&serde_json::json!({ "from": "   " }), &["from"]),
            None
        );
        assert_eq!(first_string(&serde_json::json!({}), &["from"]), None);
    }
}
