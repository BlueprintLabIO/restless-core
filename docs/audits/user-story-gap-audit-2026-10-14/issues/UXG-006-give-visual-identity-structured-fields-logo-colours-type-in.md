# UXG-006 — Give visual identity structured fields (logo, colours, type) in one place

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | medium | missing-function | US-007 |

## Problem

"Visual language" is a single free-text box. There is no place for brand colours, fonts or example images, and the logo is edited under Company → Charter, not Brand & voice. Visual work has nothing machine-readable to stay on brand with, and owners look for the logo in the wrong place.

## Evidence

- `web/src/lib/components/IdentityEditor.svelte:34-38` (visual = text hint)
- `web/src/routes/[companyId]/company/charter/+page.svelte:187-209` (name + logo live in Charter)

## Suggested direction

Add colour/font/reference-image fields under Visual language and move or mirror the logo there.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
