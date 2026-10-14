# UXG-204 — Stop a failed /goal send from creating a duplicate goal on retry

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | security | US-108 |

## Problem

"/goal <objective>" first creates the goal, then sends the message to Exec. If that send fails the composer is restored with the same text, and sending again runs the command again, creating a second identical goal. Owners who retry after a network error end up with duplicate objectives routed to teams.

## Evidence

- `web/src/lib/model/skills.ts:308-310` (addGoal then send: true)
- `web/src/lib/components/ExecutiveRail.svelte:826-838` (command runs before onask)
- `web/src/lib/components/ExecutiveRail.svelte:851-854` (composer = sent on failure)
- `web/src/lib/components/ExecutiveRail.svelte:863-865` (composer restored on throw)

## Suggested direction

Make goal creation idempotent per send (reuse the client command id) or skip re-creation when the goal already exists.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
