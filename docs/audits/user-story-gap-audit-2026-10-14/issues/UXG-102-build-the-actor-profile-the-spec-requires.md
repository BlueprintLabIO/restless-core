# UXG-102 — Build the actor profile the spec requires

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | high | spec-drift | US-059, US-061 |

## Problem

owner-cockpit §7.2/§7.3/§12.4 require role, focus, active Work, outputs, cost and model per actor. Selecting a person only opens a DM whose header is initials + name; `/people/:actor_id` does not exist.

## Evidence

- `docs/specs/owner-cockpit.md:761-790,` :1323-1335
- `web/src/lib/components/RoomConversation.svelte:1274-1286`

## Suggested direction

Add a lightweight profile pane from the DM header: role, team/lead, current Work, recent outputs.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
