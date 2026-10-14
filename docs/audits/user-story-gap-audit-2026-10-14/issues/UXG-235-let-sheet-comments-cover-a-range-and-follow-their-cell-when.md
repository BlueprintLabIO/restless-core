# UXG-235 — Let sheet comments cover a range and follow their cell when rows move

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-134 |

## Problem

A sheet comment thread stores one worksheet and one A1 address. It cannot cover a range such as a row of a budget, and when rows are inserted above, the thread stays on the old address and now points at the wrong figure; the maturity note admits this as the remaining sheet gap.

## Evidence

- `web/src/lib/components/SheetComments.svelte:155-165` (thread created with worksheet_id and a single cell)
- `web/src/lib/components/SheetComments.svelte:251-263` ("On <tab> · <cell>")
- `docs/maturity-vs-notion.md:32` (thread stays on the cell address when rows are inserted)

## Suggested direction

Store comment anchors as ranges and remap them on row/column insert and delete commands, as the engine does for formulas.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
