# UXG-088 — Refresh the sheet list and keep Library error states honest

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | ux | US-049 |

## Problem

Sheets load once per visit so new Staff sheets never appear until reload; "Sheets could not be read" has no Retry; archived lists swallow failures so Archived can look empty; kind views can flash "Nothing here yet".

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:117-136,` :190-200, :810, :1169-1173

## Suggested direction

Put sheets on the same refreshing query as docs, add Retry, show archived load failures.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
