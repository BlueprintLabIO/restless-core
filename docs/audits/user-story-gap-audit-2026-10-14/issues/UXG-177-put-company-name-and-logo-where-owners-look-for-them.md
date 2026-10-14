# UXG-177 — Put company name and logo where owners look for them

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-096 |

## Problem

Rename and logo live inside Charter, not Brand & voice, the sidebar or the overview; settings search for "logo" or "rename" finds nothing.

## Evidence

- `web/src/routes/[companyId]/company/charter/+page.svelte:187-209`
- `web/src/routes/[companyId]/company/+layout.svelte:31-44`

## Suggested direction

Add a "Name & logo" row (or top of overview) and search keywords. See also UXG-006.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
