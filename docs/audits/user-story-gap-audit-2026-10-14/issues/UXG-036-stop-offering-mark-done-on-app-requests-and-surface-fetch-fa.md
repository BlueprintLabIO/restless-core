# UXG-036 — Stop offering "Mark done" on app requests and surface fetch failures

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | consistency | — (found while tracing adjacent paths) |

## Problem

The detail says app requests have no Mark done, but the list menu and E still offer it. If `fetchAppRequests` fails the error is swallowed and the detail falls back to generic Mark done.

## Evidence

- `web/src/lib/components/InboxDetail.svelte:132-148,` :653-663
- `web/src/lib/components/InboxList.svelte:147-148,` :342-344

## Suggested direction

Hide complete-human-step for app requests everywhere; show the fetch error.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
