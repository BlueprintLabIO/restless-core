//! Reconciliation: what the receipts actually say, against what the company
//! says happened.
//!
//! Observed live (Aris, 2026-08-13): the Exec reported "5 paid, £45" while the
//! kernel held **four** successful `payment.charge` receipts — and the one
//! customer named in a receipt was the one the journal recorded as a durable
//! loss. Nothing noticed. A self-running company that reports unearned revenue
//! to its owner as fact is the exact failure the product exists to prevent.
//!
//! The split is the usual one (LLM_CURE.md frame 2). Summing receipts is
//! deterministic and belongs here. Deciding whether a prose claim contradicts
//! them is judgement and belongs to the Exec — so this does not parse the
//! journal. It computes the ledger, hands it back to the company in its next
//! wake, and lets the actor with the context do the comparing. Evidence is
//! placed in front of the claimant rather than a verdict inferred behind them.

use std::collections::BTreeMap;

use anyhow::Result;
use restless_orgintel::OrgIntel;
use serde::Serialize;

/// What the kernel's receipts record, per capability.
#[derive(Debug, Default, Serialize)]
pub struct CapabilityTally {
    pub total: usize,
    /// Receipts whose outcome carries an error, or a status the provider
    /// itself called unsuccessful. Deterministic envelope reading.
    pub failed: usize,
}

/// The receipt-backed picture of everything this company has done to the
/// world. Every number here is counted, never inferred.
#[derive(Debug, Default, Serialize)]
pub struct EffectLedger {
    pub by_capability: BTreeMap<String, CapabilityTally>,
    /// Money that moved, in minor units, only where the receipt carried BOTH
    /// an amount and a currency.
    pub money_minor: BTreeMap<String, i64>,
    /// Receipts that moved money but did not say how much, or in what
    /// currency. These are the ones nobody can reconcile — naming them is the
    /// honest alternative to guessing a unit. Observed: the same simulated
    /// provider accepted `900` and `9` for the same £9 charge.
    pub unattributable_payments: usize,
    /// Duplicate requests the idempotency guard suppressed. Counted so the
    /// protection is auditable rather than merely asserted.
    pub replays_suppressed: usize,
    /// Effects repeated on a party already acted on under a different key.
    /// Idempotency cannot catch these: two honest keys, one party.
    pub party_repeats: usize,
    /// Receipts whose status word we do not recognise. Reported rather than
    /// guessed in either direction.
    pub unknown_outcomes: usize,
    pub total: usize,
}

impl EffectLedger {
    /// One line the Exec can read in its next wake without parsing anything.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.total == 0 {
            return "no external effects recorded yet".to_string();
        }
        let mut parts: Vec<String> = self
            .by_capability
            .iter()
            .map(|(capability, tally)| {
                if tally.failed > 0 {
                    format!("{capability} {}✓/{}✗", tally.total - tally.failed, tally.failed)
                } else {
                    format!("{capability} {}", tally.total)
                }
            })
            .collect();
        for (currency, amount) in &self.money_minor {
            parts.push(format!("{} {:.2} moved", currency, *amount as f64 / 100.0));
        }
        if self.replays_suppressed > 0 {
            parts.push(format!("{} duplicate(s) suppressed", self.replays_suppressed));
        }
        if self.unknown_outcomes > 0 {
            parts.push(format!("{} receipt(s) with an unrecognised status", self.unknown_outcomes));
        }
        if self.party_repeats > 0 {
            parts.push(format!(
                "{} repeat effect(s) on a party already acted on — CHECK THESE",
                self.party_repeats
            ));
        }
        if self.unattributable_payments > 0 {
            parts.push(format!(
                "{} payment receipt(s) with no readable amount/currency",
                self.unattributable_payments
            ));
        }
        parts.join(" · ")
    }
}

/// Read every effect receipt this company has and total it up.
pub async fn effect_ledger(org: &OrgIntel) -> Result<EffectLedger> {
    let mut ledger = EffectLedger::default();
    for event in org.events_of_kind("effect").await? {
        let Some(capability) = event.body.get("capability").and_then(|v| v.as_str()) else {
            continue;
        };
        let outcome = event.body.get("outcome");
        ledger.total += 1;
        let tally = ledger.by_capability.entry(capability.to_string()).or_default();
        tally.total += 1;
        if outcome.is_some_and(|o| outcome_of(o) == Outcome::Failed) {
            tally.failed += 1;
            continue;
        }
        // Only count money for receipts that actually claim success.
        if outcome.is_some_and(|o| outcome_of(o) == Outcome::Unknown) {
            ledger.unknown_outcomes += 1;
            continue;
        }
        if capability.starts_with("payment.") {
            match money(outcome) {
                Some((currency, minor)) => {
                    *ledger.money_minor.entry(currency).or_default() += minor;
                }
                None => ledger.unattributable_payments += 1,
            }
        }
    }
    ledger.replays_suppressed = org.events_of_kind("effect_replayed").await?.len();
    ledger.party_repeats = org.events_of_kind("effect_repeat_party").await?.len();
    Ok(ledger)
}

/// What a receipt says happened. Three states, not two — an allowlist of
/// success words silently reclassifies every word a provider has not used
/// yet. Observed: `deployed` and `refunded` are successes that a
/// succeeded/delivered/sent allowlist counts as failures, which would have
/// fired a "repeating a failed approach" signal at a company deploying
/// perfectly well.
///
/// Same rule as `acp::TurnEnd` / `health::classify`, learned the same way:
/// **unknown is not failure.** A word we do not recognise is reported as
/// unrecognised, and the distinction lives in the type rather than in the
/// caller's memory of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed,
    Unknown,
}

/// One definition, shared with the organisational health signals: two notions
/// of "failed" in one codebase drift apart the moment a provider invents a
/// status word.
#[must_use]
pub fn outcome_of(outcome: &serde_json::Value) -> Outcome {
    if outcome.get("error").is_some() {
        return Outcome::Failed;
    }
    let Some(status) = outcome.get("status").and_then(|value| value.as_str()) else {
        // No status and no error: providers that simply return data. Treat as
        // succeeded — the effect surface would have carried an error otherwise.
        return Outcome::Succeeded;
    };
    let status = status.to_lowercase();
    if matches!(
        status.as_str(),
        "succeeded" | "success" | "ok" | "delivered" | "sent" | "deployed" | "refunded" | "accepted"
    ) {
        return Outcome::Succeeded;
    }
    if matches!(
        status.as_str(),
        "failed" | "declined" | "bounced" | "error" | "cancelled" | "canceled" | "rejected"
    ) || status.ends_with("_failed")
        || status.starts_with("rejected")
        || status.starts_with("error")
    {
        return Outcome::Failed;
    }
    Outcome::Unknown
}

/// Amount and currency, only when both are present and unambiguous. A missing
/// currency is not an excuse to assume one.
fn money(outcome: Option<&serde_json::Value>) -> Option<(String, i64)> {
    let outcome = outcome?;
    let amount = outcome.get("amount")?.as_i64()?;
    let currency = outcome.get("currency")?.as_str()?.to_uppercase();
    Some((currency, amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(capability: &str, outcome: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "capability": capability, "outcome": outcome })
    }

    /// Every status word both companies actually emitted. An allowlist built
    /// from imagination would have called `deployed` and `refunded` failures
    /// and fired a "repeating a failed approach" signal at a healthy company.
    #[test]
    fn real_provider_status_words_classify_correctly() {
        let succeeded = ["succeeded", "delivered", "deployed", "refunded"];
        let failed = ["failed", "declined", "bounced", "rejected_invalid_address", "refund_failed"];
        for status in succeeded {
            assert_eq!(
                outcome_of(&serde_json::json!({ "status": status })),
                Outcome::Succeeded,
                "{status} is a success"
            );
        }
        for status in failed {
            assert_eq!(
                outcome_of(&serde_json::json!({ "status": status })),
                Outcome::Failed,
                "{status} is a failure"
            );
        }
        // A word no provider has used yet is unknown, never failure.
        assert_eq!(
            outcome_of(&serde_json::json!({ "status": "queued_for_review" })),
            Outcome::Unknown
        );
        // An explicit error outranks any status.
        assert_eq!(
            outcome_of(&serde_json::json!({ "status": "succeeded", "error": "nope" })),
            Outcome::Failed
        );
    }

    /// The shapes below are verbatim from Aris's and Thymelake's real
    /// receipts, including the inconsistent payment units.
    #[test]
    fn counts_successes_failures_and_refuses_to_guess_units() {
        let events = vec![
            receipt("email.send", serde_json::json!({ "note": "delivered", "status": "sent" })),
            receipt(
                "email.send",
                serde_json::json!({ "note": "example.com is a reserved domain", "status": "bounced" }),
            ),
            receipt(
                "payment.charge",
                serde_json::json!({ "amount": 900, "status": "succeeded", "currency": "GBP" }),
            ),
            // Real receipt: succeeded, but no currency — unreconcilable.
            receipt("payment.charge", serde_json::json!({ "amount": 900, "status": "succeeded" })),
            receipt(
                "payment.charge",
                serde_json::json!({ "error": "invalid_request", "reason": "missing fields" }),
            ),
        ];
        let mut ledger = EffectLedger::default();
        for body in &events {
            let capability = body["capability"].as_str().unwrap();
            let outcome = body.get("outcome");
            ledger.total += 1;
            let tally = ledger.by_capability.entry(capability.to_string()).or_default();
            tally.total += 1;
            if outcome.is_some_and(|o| outcome_of(o) == Outcome::Failed) {
                tally.failed += 1;
                continue;
            }
            // Only count money for receipts that actually claim success.
        if outcome.is_some_and(|o| outcome_of(o) == Outcome::Unknown) {
            ledger.unknown_outcomes += 1;
            continue;
        }
        if capability.starts_with("payment.") {
                match money(outcome) {
                    Some((currency, minor)) => *ledger.money_minor.entry(currency).or_default() += minor,
                    None => ledger.unattributable_payments += 1,
                }
            }
        }
        assert_eq!(ledger.total, 5);
        assert_eq!(ledger.by_capability["email.send"].failed, 1);
        assert_eq!(ledger.by_capability["payment.charge"].failed, 1);
        // Only the receipt carrying BOTH amount and currency is counted.
        assert_eq!(ledger.money_minor["GBP"], 900);
        assert_eq!(ledger.unattributable_payments, 1);
    }
}
