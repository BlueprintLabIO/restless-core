# UXG-195 — Let the rail reach and search Exec history older than the last 100 messages

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | high | missing-function | US-105 |

## Problem

The conversation API returns at most 100 messages, and the rail's "Show earlier messages" and search operate only on that set; the full-width People view of Exec is the same component with the same cap. Once the 100 are shown the button disappears with no hint more exists, and search says "N messages in this record" as if that were everything. Only a deep link to a specific message ID reaches older history, through the room view.

## Evidence

- `crates/restless-owner/src/owner.rs:8476-8483` (human_conversation limit 100)
- `web/src/lib/components/ExecutiveRail.svelte:700-718` (window over loaded messages only)
- `web/src/lib/components/ExecutiveRail.svelte:1100-1103` (Show earlier only while windowStart > 0)
- `web/src/lib/primitives/ConversationHistoryTools.svelte:24-33` (client filter of loaded messages)
- `web/src/lib/primitives/ConversationHistoryTools.svelte:126` ("messages in this record")
- `web/src/lib/components/ExecutiveRail.svelte:151-163` (older messages reachable only via focus redirect)

## Suggested direction

Page older messages from the server on scroll-back and run conversation search server-side over the full history.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
