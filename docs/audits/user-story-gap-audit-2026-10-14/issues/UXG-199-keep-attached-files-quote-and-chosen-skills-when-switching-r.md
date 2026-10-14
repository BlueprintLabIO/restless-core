# UXG-199 — Keep attached files, quote and chosen skills when switching rail conversations

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | ux | US-106, US-107 |

## Problem

Only the text draft is saved per recipient; switching topic (General to a lead, or into and out of an Inbox focus) deliberately clears attached files, the quoted reply, selected skills and a pending new conversation, and a reload loses them too. People rooms keep files per room in memory and restore them on return. An owner who attaches a contract, glances at another conversation and comes back must re-attach.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:345-363` (files/skills/quote cleared on carryKey change)
- `web/src/lib/model/composer-options.svelte.ts:35-50` (draft stores text only)
- `web/src/lib/components/RoomConversation.svelte:351` (room keeps files per draft key)
- `web/src/lib/components/RoomConversation.svelte:363-378` (room restores files)

## Suggested direction

Hold files, quote and skills per draft key in memory as rooms do, and say when they could not survive a reload.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
