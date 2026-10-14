# UXG-203 — Confirm before /loop clear stops every loop, and let /goal clear choose the goal

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | high | ux | US-108 |

## Problem

"/loop clear" (or "/loop stop") cancels every running loop at once with no confirmation or undo, and "/goal clear" closes whichever open goal happens to be last in the list. A single short command can silently stop all of the company's recurring check-ins or close the wrong objective.

## Evidence

- `web/src/lib/model/skills.ts:164-165` (clear/stop parse)
- `web/src/lib/model/skills.ts:349-357` (Promise.all cancels all loops)
- `web/src/lib/model/skills.ts:320-326` (closes open.at(-1))
- `web/src/lib/components/ExecutiveRail.svelte:826-836` (runs immediately on send)

## Suggested direction

Ask which loop or goal to stop (or confirm "all") and offer Undo in the result.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
