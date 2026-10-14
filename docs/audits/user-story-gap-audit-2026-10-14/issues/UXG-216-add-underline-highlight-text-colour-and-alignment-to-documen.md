# UXG-216 — Add underline, highlight, text colour and alignment to document formatting

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | high | missing-function | US-121 |

## Problem

The document schema keeps only bold, italic, strikethrough, inline code and links. StarterKit's underline is turned off, the projection to the stored schema refuses any other mark, and the selection bubble offers only those five buttons. A non-technical owner writing a letter, offer or policy cannot underline a term, highlight a deadline, colour a warning or centre a heading, which every word processor they already know can do.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:1005-1010` (underline: false in StarterKit)
- `web/src/lib/model/document-editor-schema.ts:1064-1068` (projectMark allows only bold/italic/strike/code/link)
- `web/src/lib/components/DocumentEditor.svelte:679-712` (FORMATS: Bold, Italic, Strikethrough, Code)
- `services/native-documents-collaboration/src/document-codec.ts:5-12` (codec mirrors the same bounds)

## Suggested direction

Add underline, highlight (a small fixed palette) and block alignment to the company schema in both browser and codec, and show them in the selection bubble.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
