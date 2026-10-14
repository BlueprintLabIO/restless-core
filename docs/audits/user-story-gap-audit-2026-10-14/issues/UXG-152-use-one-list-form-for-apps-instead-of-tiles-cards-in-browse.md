# UXG-152 — Use one list form for Apps instead of tiles/cards in Browse

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | ux | — (found while tracing adjacent paths) |

## Problem

The same App objects are an Item list (Ask), a pseudo-table (In use) and a tile/card grid (Browse). Tiles show a name and two clamped lines and hide how it connects (sign-in vs token) in a tooltip — exactly what a chooser needs.

## Evidence

- `web/src/routes/[companyId]/apps/+page.svelte:411-437,` :446-482, :484-495
- `web/src/routes/[companyId]/apps/AppTile.svelte:219-240`

## Suggested direction

Browse as the same list rows with a visible "Sign in / Token / No sign-in" tag.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
