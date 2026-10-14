# UXG-044 — Show why blocked work is blocked in the list and detail header

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | ux | US-025 |

## Problem

Blocked rows show owner, run state and outputs but no reason; on detail the reason is only the latest attempt summary inside collapsed "Run details".

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:334-347`
- `web/src/routes/[companyId]/work/[workId]/+page.svelte:250-255,` :681-727

## Suggested direction

Lift the blocker sentence into row meta and under the detail title.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
