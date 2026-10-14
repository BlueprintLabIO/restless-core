# UXG-045 — Show and let the owner change Work priority; add sort

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | missing-function | US-031 |

## Problem

`WorkRow.priority` arrives with every Work but is never rendered or editable; lists sort only by `updated_at`. Spec §6.7 "change priorities" and §12.3 "filter by priority" are unmet.

## Evidence

- `web/src/lib/model/generated/orgintel.ts:145`
- `web/src/routes/[companyId]/work/+page.svelte:216-217,` :232, :354

## Suggested direction

Show priority on rows; let the owner set it, with Exec informed.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
