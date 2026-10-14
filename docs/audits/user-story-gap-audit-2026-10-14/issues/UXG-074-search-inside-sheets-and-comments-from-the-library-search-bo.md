# UXG-074 — Search inside sheets and comments from the Library search box

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | consistency | US-042 |

## Problem

⌘K already searches sheet cells and comments; the Library search only searches doc text and matches sheets/files by title.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:1222-1228`
- `web/src/routes/[companyId]/library/+page.svelte:704-722`
- `web/src/lib/model/sheets.ts:195`

## Suggested direction

Reuse sheet and comment search in the Library list.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
