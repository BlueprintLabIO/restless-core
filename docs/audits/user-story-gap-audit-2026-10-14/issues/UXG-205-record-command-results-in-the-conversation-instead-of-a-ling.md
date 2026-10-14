# UXG-205 — Record command results in the conversation instead of a lingering status line

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | low | ux | US-108 |

## Problem

Results of /goal and /loop (open goals, loop schedules, "Stopped 3 loops") appear only as a notice under the composer that is never cleared until the next send, and then vanishes for good. Unrelated notices such as "Link to that message copied." also linger indefinitely. The owner has no record of what a command changed.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:829-836` (command result to askNotice)
- `web/src/lib/components/ExecutiveRail.svelte:1407-1409` (notice rendered)
- `web/src/lib/components/ExecutiveRail.svelte:218` (copy notice uses same slot)
- `web/src/lib/components/ExecutiveRail.svelte:819-820` (cleared only on next send)

## Suggested direction

Post command results as a system line in the transcript and auto-dismiss transient notices.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
