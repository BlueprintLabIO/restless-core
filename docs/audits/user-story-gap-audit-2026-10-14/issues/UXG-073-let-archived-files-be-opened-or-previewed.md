# UXG-073 — Let archived files be opened or previewed

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | ux | US-041 |

## Problem

Archived files get `href: undefined`, so the owner cannot check what a file is before restoring it.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:577`

## Suggested direction

Keep the viewer link for archived files, read-only.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
