# UXG-053 — Mark "needs you" on board cards

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | ux | US-026 |

## Problem

The board passes `meta(row, false)`, suppressing the holder text for owner-waiting work with nothing in its place. In Board view an item awaiting the owner looks like any other.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:340,` :348-365
- `web/src/lib/ui/views/WorkBoard.svelte:43-57`

## Suggested direction

Add a needs-you badge or tone to the card.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
