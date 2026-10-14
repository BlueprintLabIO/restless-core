# UXG-211 — Show on a sent message which screen it was linked to

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | low | ux | US-114 |

## Problem

Each owner message may carry a context path, but no message renders it; it is only used as hidden search text. Rereading a conversation, the owner cannot tell which document or Work a terse "is this ready?" referred to.

## Evidence

- `web/src/lib/primitives/ConversationMessage.svelte:161-198` (body renders takeaway, text, attachments, details
- no context)
- `web/src/lib/primitives/ConversationHistoryTools.svelte:27` (contextPath used only in search)
- `web/src/routes/[companyId]/+layout.svelte:622-631` (contextPath sent with each message)

## Suggested direction

Show a small linked chip on the owner's message naming the page it was sent from.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
