# UXG-135 — Make app activity answer "what was it used for"

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | ux | US-075 |

## Problem

"Recent" shows at most 8 effects and 5 reads, purpose only on hover, raw actor ids, no Work links, no "see all", and no history at all when the connection is not working. "Used on N Work" is a bare count.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:67-73,` :341-369, :379

## Suggested direction

Inline purpose, display names, Work links and a paginated history regardless of status.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
