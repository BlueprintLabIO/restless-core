# UXG-109 — Let a room owner leave or hand over a group

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | missing-function | US-063 |

## Problem

`canLeave` requires role member; the owner's only exit is Archive for everyone. No ownership transfer.

## Evidence

- `web/src/lib/components/RoomManager.svelte:66-70,` :400-418

## Suggested direction

Offer "Make owner" on a participant row; allow leave after handover.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
