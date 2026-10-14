# UXG-222 — Show word and character count

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | missing-function | US-125 |

## Problem

There is no count anywhere in the editor head, the document menu or the inspector. Owners writing to a limit (a grant answer, an ad, a bio) must paste the text elsewhere to measure it.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:997-1040` (no character-count extension)
- `web/src/lib/components/DocumentEditor.svelte:998-1056` (header shows title, save state, faces and actions only)

## Suggested direction

Show words and characters for the document, and for the selection when there is one, quietly in the … menu or on hover of the save state.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
