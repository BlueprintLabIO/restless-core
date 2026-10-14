# UXG-221 — Show an outline of headings, and make in-page "#" links work

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-125 |

## Problem

Long documents (the "handbook" and "operating note" kinds exist) have no outline, table of contents or heading navigation, so the owner scrolls to find a section. The link field also accepts in-page links starting with "#", but clicking one does nothing because opening a link returns early for any "#" address, and headings carry no anchor id to land on.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:381` (safeHref accepts "#…")
- `web/src/lib/model/document-editor-schema.ts:399` (linkFromInput keeps "#…")
- `web/src/lib/components/DocumentEditor.svelte:847-849` (openHref ignores "#" links)
- `web/src/lib/model/documents.ts:3-11` (handbook / operating_note kinds)

## Suggested direction

Add an outline panel built from headings that scrolls to each one, and resolve "#" links to a heading's block, or stop accepting them.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
