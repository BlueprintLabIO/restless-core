# UXG-090 — Offer "Discuss with Exec" on sheets and link the file on file Discuss

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | consistency | US-050, US-051 |

## Problem

Sheets have no Discuss; file Discuss opens an empty draft with no reference to the file, unlike document Discuss which links the doc.

## Evidence

- `web/src/routes/[companyId]/library/sheets/+page.svelte` (no Discuss)
- `web/src/lib/components/LibraryViewer.svelte:239-244`
- `web/src/routes/[companyId]/library/documents/+page.svelte:284-289`

## Suggested direction

Give sheets and files the same Discuss: link in the draft plus an access grant.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
