# UXG-224 — Allow resizing, describing and captioning images

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-126 |

## Problem

The image node stores a width and alt text, but nothing in the editor sets either: alt is derived from the file name ("IMG_2041") and width is never written, so every picture renders at its natural size. There is no caption or alignment. Screen-reader users get file names, and a wide screenshot cannot be shrunk.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:777-794` (width and alt attributes exist)
- `web/src/lib/components/DocumentEditor.svelte:584-588` (alt from imageAltFromName only)
- `web/src/lib/model/document-editor-schema.ts:736-743` (imageAltFromName)
- `web/src/lib/model/document-editor-schema.ts:1135-1162` (projection accepts width/alt)

## Suggested direction

When an image is selected, show a small bar with size presets / drag handle, an editable description and an optional caption.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
