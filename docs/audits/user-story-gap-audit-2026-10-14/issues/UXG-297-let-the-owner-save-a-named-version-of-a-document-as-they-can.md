# UXG-297 — Let the owner save a named version of a document as they can a sheet

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | consistency | US-184 |

## Problem

Sheets have a Save version button and a checkpoint list. Documents do not, although the owner API accepts POST to the document versions route to create a named version and the CLI exposes `restless document checkpoint`. The web client even declares `CreateDocumentVersionInput` but never sends it. Document history therefore only holds versions created by restores, reviews or agents, and an owner cannot mark "the version I sent".

## Evidence

- `crates/restless-owner/src/owner_documents.rs:87-89` (versions route has GET and POST create_document_version)
- `crates/restless-owner/src/owner_documents.rs:1416-1440` (create_named_document_version)
- `web/src/lib/model/documents.ts:329-330` (CreateDocumentVersionInput declared, never used)
- `web/src/lib/model/documents.ts:500` (only GET versions is called)
- `web/src/routes/[companyId]/library/sheets/+page.svelte:715` (sheets have Save version)
- `crates/restless/src/document.rs:97-98` (CLI checkpoint)

## Suggested direction

Add "Save version" with an optional name to the document inspector's history, posting to the existing versions route.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
