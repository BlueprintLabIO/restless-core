# UXG-208 — Say "Seen, no reply" instead of "Replied" when Exec's turn ends without a message

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | ux | US-112 |

## Problem

When a turn completes without producing a message (Exec decided to act quietly) the dock still reads "Replied" while the receipt beneath says "Seen by Exec". The two lines contradict each other and the owner scrolls looking for a reply that does not exist.

## Evidence

- `web/src/lib/primitives/ConversationTurnDock.svelte:92-94` (complete always "Replied")
- `web/src/lib/model/queries.svelte.ts:462-468` (turn completes with completedMessageId null)
- `web/src/lib/components/ExecutiveRail.svelte:586-590` (receipt "Seen by" for the same state)

## Suggested direction

Distinguish a completed turn with no message and say what Exec did instead, linking any Work it touched.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
