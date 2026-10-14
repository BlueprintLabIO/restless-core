# UXG-225 — Extend document tables: rows above, columns left, header row, widths

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-127 |

## Problem

A table is inserted as a fixed two-by-two grid with a heading row, and the only actions are add a row below, add a column to the right, and delete row / column / table. There is no way to insert above or to the left, to toggle the header row, to resize columns or to merge cells. Building a simple price list needs workarounds such as adding at the end and retyping.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:100-101` (DocumentTableAction list)
- `web/src/lib/model/document-editor-schema.ts:986-995` (documentTable is 2x2)
- `web/src/lib/components/DocumentEditor.svelte:1058-1090` (table bar buttons)

## Suggested direction

Add row above / column left, a header-row toggle and column resizing; offer "Open as sheet" for tables that outgrow a document.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
