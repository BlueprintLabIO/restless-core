# UXG-022 — Give "Set aside" dates, the asker, and a visible failure state

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | ux | US-012 |

## Problem

Set-aside rows show only a truncated title and Restore: no date, no asker, no search. If loading fails, the fold silently does not appear.

## Evidence

- `web/src/lib/components/SetAside.svelte:31-35,` :59-77

## Suggested direction

Add relative time and asker; say plainly when the list cannot be read.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
