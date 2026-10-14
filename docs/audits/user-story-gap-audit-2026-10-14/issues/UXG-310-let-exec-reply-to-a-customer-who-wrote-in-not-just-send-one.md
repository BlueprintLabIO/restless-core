# UXG-310 — Let Exec reply to a customer who wrote in, not just send one first-contact email

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | high | missing-function | US-197 |

## Problem

The email mandate refuses any send to a recipient who already has a sent or uncertain email under it, or any successful customer-contact email at all. While a mandate is active, the generic Resend path is also closed. So once Exec has replied to a customer, it cannot answer that customer's follow-up by any route, and without a mandate every new correspondent becomes a "First contact" approval. The "a customer emailed, handle it" journey has no path past the first reply, so support cannot run on its own.

## Evidence

- `crates/restless-engine/src/mandate.rs:593-620` (refuses recipient with earlier sent/uncertain mandate email)
- `crates/restless-engine/src/mandate.rs:621-638` (refuses recipient with any successful customer-contact.email receipt)
- `crates/restless-engine/src/effect.rs:262-281` (active mandate closes generic Resend binding)
- `docs/exec-email-mandates.md:5-7` (no earlier outreach to same recipient
- generic path closed)
- `crates/restless-engine/src/inbound.rs:1-13` (inbound mail reaches Exec)

## Suggested direction

Add a reply allowance alongside outreach, bounded to correspondents who wrote in first (tied to the inbound receipt), so Exec can keep a support thread going without an approval per message.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
