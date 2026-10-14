# UXG-217 — Keep images, formatting and merged cells when pasting from Word or Google Docs

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | high | missing-function | US-122 |

## Problem

Pasted HTML is held to the company schema, so an image pasted inside Word or Google Docs content (an img that is not a Restless figure) is dropped, underline and colour are dropped, list marker styles are dropped and tables lose merged cells because the grid has no colspan. Nothing tells the owner what was lost. Moving existing company writing into Restless is the first thing an owner does, and silent loss of pictures and layout makes the editor look broken. Distinct from UXG-080 (turning a .docx upload into a document): this is clipboard paste inside the editor.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:796-801` (image parses only figure[data-attachment-id])
- `web/src/lib/model/document-editor-schema.ts:103-104` (tables are plain grids, no merged cells)
- `web/src/lib/model/document-editor-schema.ts:491-493` (Word / Google Docs list marker types dropped)
- `web/src/lib/model/document-editor-schema.ts:1007` (underline off)
- `web/src/lib/model/document-editor-schema.ts:824-833` (only clipboard image files, not images inside pasted HTML, are uploaded)

## Suggested direction

On paste, upload images found in the pasted HTML (data: or fetchable URLs) as attachments, expand merged cells, and show a one-line note listing anything that could not be kept.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
