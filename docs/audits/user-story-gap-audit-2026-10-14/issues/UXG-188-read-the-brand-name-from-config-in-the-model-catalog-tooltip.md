# UXG-188 — Read the brand name from config in the model-catalog tooltip

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | consistency | — (found while tracing adjacent paths) |

## Problem

The Intelligence tooltip hard-codes "Restless checks models.dev hourly…", breaking brand-neutral code.

## Evidence

- `web/src/lib/components/CompanyProvider.svelte:1113`

## Suggested direction

Use PRODUCT_NAME from $lib/brand/brand.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
