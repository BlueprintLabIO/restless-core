# UXG-233 — Offer templates and use the document kinds when starting a document

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-133 |

## Problem

Documents have kinds (brief, plan, decision note, report, review, handbook, operating note) but nothing in the web sets one: New document always creates an untitled "freeform" page, and no screen changes the kind afterwards. There are no templates or "start from a copy" choices, so owners re-create the same structure for every brief or decision note, and the Library cannot group by kind meaningfully.

## Evidence

- `web/src/lib/model/documents.ts:3-11` (DocumentKind list)
- `web/src/lib/components/LibraryNew.svelte:64-79` (always kind: 'freeform')
- `web/src/lib/components/DocumentEditor.svelte:132` (kind held, with kindDirty, but no control renders it)

## Suggested direction

Offer a short template picker on New (blank plus a few kinds with headings prefilled) and let Exec draft a starting outline; let the kind be changed from the … menu.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
