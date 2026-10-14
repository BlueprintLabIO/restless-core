# UXG-059 — Reorder or replace the board on mobile

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | mobile | US-037 |

## Problem

Under 760px the columns stack Next → In motion → Waiting → Done with Done unbounded; blocked items are buried below all queued and active cards.

## Evidence

- `web/src/lib/ui/views/WorkBoard.svelte:183-188`
- `web/src/routes/[companyId]/work/+page.svelte:349`

## Suggested direction

Put blocked first and fold Done on narrow panes, or force List.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
