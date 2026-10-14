# UXG-014 — Let an applied-change row open the change it made

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | spec-drift | US-015 |

## Problem

The spec says the For you row "opens the change" (document diff or before/after). Applied rows have no link; clicking only marks them read. You can Undo a change you cannot see.

## Evidence

- `web/src/lib/components/ForYou.svelte:141-145`
- `web/src/lib/components/AppliedChanges.svelte:47-73`

## Suggested direction

Link each row to its target with the version comparison.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
