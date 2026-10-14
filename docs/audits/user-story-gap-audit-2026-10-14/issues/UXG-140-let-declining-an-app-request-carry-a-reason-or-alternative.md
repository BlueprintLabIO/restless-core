# UXG-140 — Let declining an app request carry a reason or alternative

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | missing-function | US-078 |

## Problem

"Not needed"/"×" only dismiss; Exec learns nothing and may ask again or stay stuck.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:484-497`
- `web/src/lib/components/AppRequestCards.svelte:98-107`

## Suggested direction

Optional one-line reason or "use X instead".

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
