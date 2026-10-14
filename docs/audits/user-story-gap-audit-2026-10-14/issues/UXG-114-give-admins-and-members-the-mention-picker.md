# UXG-114 — Give admins and members the @mention picker

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | high | missing-function | US-066 |

## Problem

The person picker comes from `referenceOptions`, included only when `ownerAccess`. A cofounder typing "@Ines" mentions nobody (tokens must be `@<actor_id>`) and gets no feedback.

## Evidence

- `web/src/lib/components/RoomConversation.svelte:878-918,` :1741-1750
- `web/src/lib/model/composer-options.svelte.ts:90-97`

## Suggested direction

Always supply room people as mention options, independent of owner access.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
