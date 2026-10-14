# UXG-236 — Allow pictures in sheets

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | missing-function | US-135 |

## Problem

The spreadsheet model is created with only a transport, the client identity and a mode; no file store is given to the engine, so its image insert has nowhere to put a picture. Owners cannot add a product photo or logo to an inventory or price sheet, although documents can hold images.

## Evidence

- `web/src/lib/model/sheet-editor.ts:449-456` (new Model(snapshot, { transportService, client, mode }) only)
- `web/src/lib/components/DocumentEditor.svelte:558-603` (documents upload images as company attachments)

## Suggested direction

Give the engine a file store backed by company attachments, or hide image insert with a clear note.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
