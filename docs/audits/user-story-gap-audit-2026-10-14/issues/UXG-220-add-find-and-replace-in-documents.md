# UXG-220 — Add find and replace in documents

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-125 |

## Problem

The editor has no search or replace extension and no shortcut for it; the only find is the browser's own, which cannot replace and does not reach text that has not rendered yet. Renaming a product, a client or a price across a handbook means hunting by eye. Sheets get Find and Replace from their engine, so the two editors also disagree.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:997-1040` (extension list: no search/replace)
- `web/src/lib/components/DocumentEditor.svelte:754-767` (formatKeys handles only the comment shortcut)

## Suggested direction

Add a find bar (⌘F inside the editor, ⌘⇧H replace) that walks matches through the live document as ordinary collaborative edits.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
