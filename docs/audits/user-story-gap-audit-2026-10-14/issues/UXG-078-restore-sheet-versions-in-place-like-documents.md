# UXG-078 — Restore sheet versions in place, like documents

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | consistency | US-045 |

## Problem

Docs offer "Restore as new version"; sheets only "Recover as a copy", creating "Title — recovered" while the original keeps the bad edits and its links.

## Evidence

- `web/src/routes/[companyId]/library/sheets/+page.svelte:538-549,` :889

## Suggested direction

Offer in-place restore that checkpoints first, as CSV import already does.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
