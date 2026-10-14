# UXG-081 — Export documents as Word (.docx)

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | missing-function | US-048 |

## Problem

Document export is Markdown or the print dialog; owners sending contracts usually need .docx. Sheets already export .xlsx.

## Evidence

- `web/src/routes/[companyId]/library/documents/+page.svelte:563-568`

## Suggested direction

Add DOCX download next to Markdown.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
