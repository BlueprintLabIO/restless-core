# UXG-075 — Sort and count across all documents, not just loaded pages

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | ux | US-043 |

## Problem

Documents are paged, but sorting and sidebar counts apply only to loaded rows: "A to Z" can miss page 2 and counts undercount.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:664-672,` :770-784, :1312-1316

## Suggested direction

Sort on the server or load all before sorting; show server counts.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
