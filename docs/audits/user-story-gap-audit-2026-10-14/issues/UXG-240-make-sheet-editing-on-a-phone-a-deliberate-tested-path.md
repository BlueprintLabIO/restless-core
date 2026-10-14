# UXG-240 — Make sheet editing on a phone a deliberate, tested path

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | mobile | US-136 |

## Problem

Restless adds no phone adaptation to the sheet editor; the maturity note says touch editing "is the engine's own". The grid keeps a 320 px minimum height, the comments panel covers it at narrow widths, and nothing was checked for entering formulas or selecting ranges by touch. Distinct from UXG-151/187 (Apps and settings on mobile).

## Evidence

- `docs/maturity-vs-notion.md:37` (touch editing of sheets is the engine's own)
- `web/src/lib/components/SheetEditor.svelte:166-172` (min-height 320px)
- `web/src/routes/[companyId]/library/sheets/+page.svelte:1170-1204` (layout-only media rules)

## Suggested direction

Run a phone walk of the common edits (type, formula, select range, add row, comment) and fix or document what fails; offer a simple row-form view if needed.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
