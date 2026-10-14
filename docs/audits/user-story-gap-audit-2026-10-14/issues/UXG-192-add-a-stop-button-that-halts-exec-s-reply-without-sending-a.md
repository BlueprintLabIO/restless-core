# UXG-192 — Add a Stop button that halts Exec's reply without sending a new message

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | high | missing-function | US-103, US-177 |

## Problem

While Exec is working, the only way to stop it is the Interrupt button, which is disabled until the composer holds sendable text, and it always sends that text as a new message. An owner who simply wants Exec to stop (wrong direction, runaway tool loop) has to invent a message. The owner API has an interrupt route but it only cancels an unread queued message and the SPA never calls it.

## Evidence

- `web/src/lib/primitives/Composer.svelte:532-547` (Interrupt is a submit button, disabled={!sendable})
- `web/src/lib/components/ExecutiveRail.svelte:1364` (interruptible only paired with send)
- `web/src/routes/[companyId]/+layout.svelte:612-640` (interrupt only via railConversation.send)
- `crates/restless-owner/src/owner.rs:1506-1507` (conversation/{message_id}/interrupt route)
- `crates/restless-owner/src/owner.rs:9390-9395` (only cancels an unread ordinary message)

## Suggested direction

Show a Stop control in the turn dock / composer while working that cancels the turn with no new message, and say what was kept.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
