# UXG-207 — Show live progress when talking to a Staff member or an agent in a group room

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | consistency | US-111 |

## Problem

In People, the live turn dock is followed only for accountable direct conversations (Exec or a team lead with owner access). A DM with a Staff worker, any group room with an agent, and a member's DM with Exec show no "working" state at all, and skills are not offered in those composers. The same Staff topic opened in the rail does show progress, so behaviour depends on where the owner opened the chat.

## Evidence

- `web/src/lib/components/RoomConversation.svelte:588-593` (accountableDirect definition)
- `web/src/lib/components/RoomConversation.svelte:594-605` (followLiveActivity = accountableDirect)
- `web/src/lib/components/RoomConversation.svelte:615-619` (skills enabled only when accountableDirect)
- `web/src/lib/components/RoomConversation.svelte:630` (leadTurn null otherwise)
- `web/src/lib/components/RoomConversation.svelte:1567-1569` (dock rendered only for leadTurn)

## Suggested direction

Follow the agent's activity in any conversation where an agent is replying, and offer skills wherever an agent is addressed.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
