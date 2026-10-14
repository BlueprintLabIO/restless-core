# UXG-057 — Unify status words: Waiting / Waiting on you / Blocked / Needs you

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | consistency | US-025 |

## Problem

`blocked` is "Waiting" in the list, board and pill; the sidebar "Waiting on you" means owner action; the group is "Needs you"; the rail says "Blocked". Owners confuse blocked work with work waiting on them.

## Evidence

- `web/src/lib/work/status.ts:6-12`
- `web/src/routes/[companyId]/work/+page.svelte:221,` :450
- `web/src/lib/components/WorkInFlight.svelte:54-55`

## Suggested direction

One label per state (e.g. "Blocked" and "Needs you") everywhere.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
