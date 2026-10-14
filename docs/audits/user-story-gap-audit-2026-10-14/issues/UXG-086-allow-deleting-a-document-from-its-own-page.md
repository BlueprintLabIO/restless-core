# UXG-086 — Allow deleting a document from its own page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | ux | US-041 |

## Problem

The doc ⋯ menu offers Archive/Restore but no delete even when archived; the owner must go to Library → Archived → find the row.

## Evidence

- `web/src/routes/[companyId]/library/documents/+page.svelte:594-601`

## Suggested direction

Add "Delete permanently…" to an archived document's menu.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
