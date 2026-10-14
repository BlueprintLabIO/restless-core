# UXG-068 — Add row quick actions (and later bulk actions/export) to the Work list

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | missing-function | — (found while tracing adjacent paths) |

## Problem

`Item` supports an `actions` snippet but the Work list passes none: no quick open/discuss/stop, no multi-select, no export.

## Evidence

- `web/src/lib/ui/page/Item.svelte` (actions prop)
- `web/src/routes/[companyId]/work/+page.svelte:407-422`

## Suggested direction

Add per-row quick actions first; bulk/export only if dogfood demands.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
