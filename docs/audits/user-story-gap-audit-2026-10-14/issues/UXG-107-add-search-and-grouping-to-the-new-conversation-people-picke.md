# UXG-107 — Add search and grouping to the new-conversation people picker

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | ux | US-062 |

## Problem

New conversation lists everyone as a flat checkbox list with no filter or team grouping, which breaks down with elastic Staff pools.

## Evidence

- `web/src/lib/components/RoomManager.svelte:433-441`

## Suggested direction

Add a filter and group by team.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
