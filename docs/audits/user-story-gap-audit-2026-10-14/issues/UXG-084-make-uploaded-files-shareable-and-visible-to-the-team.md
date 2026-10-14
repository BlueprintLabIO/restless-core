# UXG-084 — Make uploaded files shareable and visible to the team

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | missing-function | US-047 |

## Problem

"From you" rows are filtered to the uploader and have no Share/Archive/Rename; only owners can upload, 5 MB limit. A file cannot be given to a teammate from the Library.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:160-163,` :853, :971-980, :1285-1292
- `web/src/lib/model/attachments.ts:8`

## Suggested direction

Treat kept files as Library items with sharing, or explain scope in a tooltip.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
