# UXG-200 — Persist the rail's retry key for an uncertain send across reloads and topic switches

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | security | US-107 |

## Problem

When a rail send's outcome is unknown the owner is told "Try sending it again", and the conversation query keeps the command id in memory so a retry is idempotent. That key lives only in the query object: a reload or topic switch drops it, so resending creates a second message, attachment set and wake. People rooms store the command id with the draft and label the button "Retry send".

## Evidence

- `web/src/lib/model/queries.svelte.ts:425-434` (uncertainCommand in memory)
- `web/src/lib/model/queries.svelte.ts:563-583` (reuse only within the same query instance)
- `web/src/routes/[companyId]/+layout.svelte:654-660` ("Try sending it again")
- `web/src/lib/model/composer-options.svelte.ts:42-50` (rail draft saves text only)
- `web/src/lib/components/RoomConversation.svelte:1067-1070` (room persists commandId with draft)
- `web/src/lib/components/RoomConversation.svelte:1754` ("Retry send" label)

## Suggested direction

Save the uncertain command id with the rail draft and show "Retry send" until the server confirms.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
