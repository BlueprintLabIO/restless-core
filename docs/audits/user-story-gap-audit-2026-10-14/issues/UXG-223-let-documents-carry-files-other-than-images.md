# UXG-223 — Let documents carry files other than images

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-126 |

## Problem

Only PNG, JPEG, GIF and WebP can go into a document; dropping or pasting a PDF, spreadsheet or contract is ignored, and the "/" menu has only Image. An owner cannot attach the signed lease to the lease summary or the quote to a brief, even though the company already stores such files in the Library. There are no embeds (video, map, a sheet) either.

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:558-564` (non-image files refused with "Only PNG, JPEG, GIF and WebP")
- `web/src/lib/model/document-editor-schema.ts:732` (DOCUMENT_IMAGE_TYPES)
- `web/src/lib/model/document-editor-schema.ts:824-841` (paste/drop handle image files only)
- `web/src/lib/components/DocumentBlockMenus.svelte:119-125` (no file or embed insert)

## Suggested direction

Add a file block that names a company attachment and opens it in the Library preview, and consider a sheet/link embed block later.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
