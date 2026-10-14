# UXG-072 — Allow permanent deletion of archived Work outputs

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | missing-function | US-041 |

## Problem

Archived Work files can only be restored; "Delete permanently…" exists only for native items, so obsolete outputs build up forever.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:899-908,` :1293-1296

## Suggested direction

Add artifact delete, or explain plainly why outputs are kept.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
