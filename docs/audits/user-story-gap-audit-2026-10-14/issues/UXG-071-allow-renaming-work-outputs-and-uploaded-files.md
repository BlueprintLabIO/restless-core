# UXG-071 — Allow renaming Work outputs and uploaded files

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | missing-function | US-039, US-047 |

## Problem

Rename appears only for native docs and sheets; Staff outputs and uploads keep labels like `IMG_2031.jpg`.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:1271-1274`

## Suggested direction

Allow a display-label rename for artifacts and kept files.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
