# UXG-065 — Make goal rows real links in the Work sidebar

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | consistency | — (found while tracing adjacent paths) |

## Problem

Fixed views are `href` links but goal rows are toggle buttons: they cannot open in a new tab or be copied, and clicking the active goal jumps back to All work.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:108,` :380, :487, :501

## Suggested direction

Render goal rows with `viewHref(goal.id)`; drop the toggle-off behaviour.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
