# UXG-060 — Cap or fold the board's Done column

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | ux | US-037 |

## Problem

The List folds Done but the Board renders every completed item ever; the unused `footer` slot was meant for history.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:349-355,` :609, :619-627
- `web/src/lib/ui/views/WorkBoard.svelte:32-33,` :64

## Suggested direction

Show recent Done with a "show all" footer.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
