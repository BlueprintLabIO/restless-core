# UXG-062 — Replace hover-only title explanations on Work with touch-reachable tips

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | a11y | — (found while tracing adjacent paths) |

## Problem

"Done · unverified", Lead/Staff/Quality bar terms, the board's "R2" and goal metadata are explained only by `title`, which goal.ts itself says never reaches touch screens.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:431-433,` :757, :760, :770-772
- `web/src/lib/ui/views/WorkBoard.svelte:55`
- `web/src/routes/[companyId]/work/+page.svelte:386-396`
- `web/src/lib/work/goal.ts:20-21`

## Suggested direction

Use InfoTip; spell out "Revision 2".

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
