# UXG-232 — Let writers save and name a version from the editor

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-133 |

## Problem

The owner API accepts a named document version with a reason, and the editor even has a projection helper for it, but no web client calls it; document versions appear only from automatic saves and reviews. Sheets do offer Save version, but name it with the current date and time with no chance to type "Sent to bank". Owners cannot mark a milestone they will want to find again. Distinct from UXG-077/078/082 (viewing, restoring and downloading versions).

## Evidence

- `crates/restless-owner/src/owner_documents.rs:87-90` (POST /documents/{document}/versions)
- `crates/restless-owner/src/owner_documents.rs:1416-1440` (create_named_document_version with reason)
- `web/src/lib/model/document-editor-schema.ts:1198-1205` (projectDocumentContent, used only by tests)
- `web/src/routes/[companyId]/library/sheets/+page.svelte:399-411` (Save version titled new Date().toLocaleString())

## Suggested direction

Add "Name this version…" to the document … menu and History, and let sheet Save version take a name.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
