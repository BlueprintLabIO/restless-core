# UXG-091 — Hide or explain "Request review…" for people who cannot request one

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | ux | US-053 |

## Problem

The item is shown to everyone; without edit access the panel just says "No active review." with no action.

## Evidence

- `web/src/routes/[companyId]/library/documents/+page.svelte:593`
- `web/src/lib/components/DocumentInspector.svelte:1612-1630`

## Suggested direction

Show only to editors, or explain who can request a review.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
