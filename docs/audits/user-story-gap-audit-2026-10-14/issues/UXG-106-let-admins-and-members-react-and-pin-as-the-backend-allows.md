# UXG-106 — Let admins and members react and pin, as the backend allows

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | consistency | — (found while tracing adjacent paths) |

## Problem

`onreact`/`onpin` are passed only when `ownerAccess`; the server accepts any principal who can read the message.

## Evidence

- `web/src/lib/components/RoomConversation.svelte:82,` :1478-1480, :1505-1507, :1677-1679
- `crates/restless-owner/src/owner.rs:10646-10676`

## Suggested direction

Gate reactions and pins on room membership, not owner role.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
