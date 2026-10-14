# UXG-226 — Let blocks be moved, duplicated and deleted without the hover grip

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | a11y | US-128 |

## Problem

Moving a block is drag-only through a grip that appears on mouse hover, and the grip is hidden entirely on touch screens; the block menu has no Move up / Move down. On a phone the owner therefore cannot move, duplicate, delete or copy a link to a block at all, and keyboard users can open the menu with Ctrl+/ but still cannot reorder. Distinct from UXG-061 (Work board semantics).

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:1864-1869` (block handle hidden for hover: none)
- `web/src/lib/model/document-editor-schema.ts:938-984` (drag handle is the only move path)
- `web/src/lib/components/DocumentBlockMenus.svelte:482-520` (menu: Comment, Copy link, Duplicate, Turn into, Delete)

## Suggested direction

Add Move up / Move down to the block menu with Alt+Shift+Arrow shortcuts, and open the block menu from a long-press or a tap on a small gutter button on touch.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
