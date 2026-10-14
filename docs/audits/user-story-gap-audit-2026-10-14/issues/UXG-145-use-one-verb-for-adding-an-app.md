# UXG-145 — Use one verb for adding an app

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | consistency | — (found while tracing adjacent paths) |

## Problem

The same act is "Add", "Turn on", "Add {name}" and "Allow" in different places.

## Evidence

- `web/src/routes/[companyId]/apps/AppTile.svelte:236`
- `web/src/routes/[companyId]/apps/+page.svelte:349,` :393
- `web/src/lib/components/AppRequestCards.svelte:95`
- `web/src/routes/[companyId]/apps/[app]/+page.svelte:306,` :519

## Suggested direction

One verb pair (Add/Remove) everywhere.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
