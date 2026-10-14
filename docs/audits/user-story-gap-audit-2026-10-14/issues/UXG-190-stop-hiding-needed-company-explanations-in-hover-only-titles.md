# UXG-190 — Stop hiding needed Company explanations in hover-only titles

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | a11y | — (found while tracing adjacent paths) |

## Problem

The spend line's "running total, not monthly" note, the weekday time zone and Empty compact info live only in `title` attributes.

## Evidence

- `web/src/routes/[companyId]/company/+page.svelte:47-50`
- `web/src/lib/ui/page/Empty.svelte:36`
- `web/src/routes/[companyId]/company/schedules/+page.svelte:221`

## Suggested direction

Route these through InfoTip.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
