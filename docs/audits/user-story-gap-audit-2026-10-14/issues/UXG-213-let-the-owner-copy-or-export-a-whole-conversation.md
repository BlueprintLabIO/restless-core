# UXG-213 — Let the owner copy or export a whole conversation

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | low | missing-function | US-116 |

## Problem

Messages can be copied one at a time and linked individually, but there is no way to copy a range, a thread of decisions, or the full transcript with Exec to share with an adviser or paste into a document.

## Evidence

- `web/src/lib/primitives/ConversationMessage.svelte:111-117` (per-message copy only)
- `web/src/lib/components/ExecutiveRail.svelte:1148-1165` (More menu: pin, copy link, turn into Work)
- `web/src/lib/primitives/ConversationHistoryTools.svelte:82-92` (header tool: search only)

## Suggested direction

Add "Copy conversation" / "Save to Library" to the rail header covering the loaded range or a selected day.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
