# UXG-160 — Provide a way back into an unfinished welcome conversation

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-086 |

## Problem

The read-back that writes the charter and name exists only on `/{id}/welcome`, and nothing links back to it. An owner who clicks ← Home returns to a nameless, charterless company and must write the charter by hand.

## Evidence

- `web/src/routes/[companyId]/welcome/+page.svelte:217`
- `web/src/routes/[companyId]/+layout.svelte:523`
- `web/src/lib/model/company-setup-rows.svelte.ts:207`

## Suggested direction

Point the "Write the charter" to-do (and charterless companies) back to the welcome conversation.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
