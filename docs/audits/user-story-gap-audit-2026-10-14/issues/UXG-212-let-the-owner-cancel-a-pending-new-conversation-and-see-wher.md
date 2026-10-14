# UXG-212 — Let the owner cancel a pending new conversation and see where earlier ones began

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | low | ux | US-115 |

## Problem

"New conversation" sets a pending focus with no way to cancel it short of switching topics, is disabled while Exec works with no explanation, and only the current focus draws a boundary. Earlier "new conversation" starts leave no marker, so the owner cannot find where a previous topic began.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:782-794` (beginNewFocus, no cancel path)
- `web/src/lib/components/ExecutiveRail.svelte:984-990` (disabled while working, title unchanged)
- `web/src/lib/components/ExecutiveRail.svelte:774-780` (boundary only for active focus)

## Suggested direction

Add a cancel on the pending boundary, explain the disabled state, and mark past focus starts in the transcript.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
