# UXG-230 — Say plainly when a document is too large to save, and warn before it gets there

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | ux | US-131 |

## Problem

Documents are capped at 1 MB of JSON, 20,000 nodes and 500,000 characters. Past any cap the store marks the whole body invalid and the editor says the latest change "holds something documents can't keep. Undo it", which does not mention size and may not be fixed by one Undo; every edit after that is unsaved. There is no size indicator or warning as the limit approaches.

## Evidence

- `services/native-documents-collaboration/src/document-codec.ts:9-12` (size caps)
- `services/native-documents-collaboration/src/document-codec.ts:236-264` (any cap → invalid)
- `services/native-documents-collaboration/src/server.ts:440-457` (broadcasts invalid)
- `web/src/lib/components/DocumentEditor.svelte:1131-1136` (generic "can't keep" message)

## Suggested direction

Return a distinct "too large" save status with the measure, warn at ~80% of a cap, and suggest splitting into a second document.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
