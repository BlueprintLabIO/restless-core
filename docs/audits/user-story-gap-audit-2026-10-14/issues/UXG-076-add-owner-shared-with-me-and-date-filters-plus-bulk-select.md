# UXG-076 — Add owner, "shared with me" and date filters, plus bulk select

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | missing-function | US-043 |

## Problem

Views are only kinds, review, pinned, yours and archived. No "what Exec made this week", no "shared with me", and no multi-select to archive or pin several items.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:811-817,` :1223-1300

## Suggested direction

Add owner/date facets and a selection mode for batch archive and pin.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
