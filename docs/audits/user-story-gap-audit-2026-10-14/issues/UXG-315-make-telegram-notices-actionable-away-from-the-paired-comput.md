# UXG-315 — Make Telegram notices actionable away from the paired computer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | mobile | US-202 |

## Problem

Telegram carries buttons only for approvals and mandate proposals. Reviews, human steps and payments arrive as a link. When pairing happened from a local cockpit, that link is prefixed "Opens only on the computer you paired from", so on a phone at 2am or on holiday the owner can read the notice but cannot act on it. Distinct from UXG-117/118 (pairing and filtering).

## Evidence

- `crates/restless-engine/src/telegram.rs:398-427` (bindings, and therefore buttons, only for approval_required and email_mandate_proposal)
- `crates/restless-engine/src/telegram.rs:428-446` (link-only otherwise
- loopback warning)
- `crates/restless-engine/src/telegram.rs:465-470` (loopback_origin)

## Suggested direction

Offer the reachable network entry when pairing, and give simple review items Accept or Snooze buttons that use the same bound-decision path as approvals.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
