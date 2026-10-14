# UXG-191 — Add Retry to a failed Exec reply

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | high | missing-function | US-102 |

## Problem

When a turn fails the dock only says "Exec couldn't reply: …" and the reason text tells the owner to "Send it again". There is no Retry control on the failed turn, so the owner must retype or copy their own message, which also re-sends any quote or attachments by hand. The expanded detail additionally prints the raw runtime error string (transport and ACP jargon) to the owner. For a product that "runs the business for you", a provider blip should be one click to recover, not a manual redo.

## Evidence

- `web/src/lib/primitives/ConversationTurnDock.svelte:92-106` (complete/failed status only, no action)
- `web/src/lib/primitives/ConversationTurnDock.svelte:254-256` (raw turn.live.error rendered)
- `web/src/lib/model/turn-failure.ts:18-31` ("Send it again" advice)
- `web/src/lib/components/ExecutiveRail.svelte:1306` (dock is the only failure surface)

## Suggested direction

Offer a Retry on the failed dock that re-wakes the same turn for the same message (and keep raw errors behind a "details" disclosure).

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
