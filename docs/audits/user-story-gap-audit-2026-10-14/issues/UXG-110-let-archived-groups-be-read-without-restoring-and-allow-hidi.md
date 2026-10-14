# UXG-110 — Let archived groups be read without restoring, and allow hiding DMs

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | ux | US-063 |

## Problem

Archived rows offer only Restore; non-owner participants see the room vanish; DMs can never be hidden so Chats only grows.

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:615-636`
- `web/src/lib/model/rooms.ts:662-665`
- `web/src/lib/components/RoomManager.svelte:61-64`

## Suggested direction

Open archived rooms read-only; add "Hide chat" for DMs.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
