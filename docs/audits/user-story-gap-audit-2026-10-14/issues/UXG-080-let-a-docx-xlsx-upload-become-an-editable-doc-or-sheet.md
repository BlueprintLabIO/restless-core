# UXG-080 — Let a .docx/.xlsx upload become an editable doc or sheet

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | high | missing-function | US-042, US-049 |

## Problem

Uploaded Excel/Word files become read-only previews with no "Open as sheet/document". The sheet import page (`?import=1`) is never linked, and `POST .../documents/{id}/imports/markdown` has no UI caller.

## Evidence

- `web/src/lib/components/LibraryNew.svelte` (upload only)
- `web/src/routes/[companyId]/library/sheets/+page.svelte:8` (only reference to import=1)
- `crates/restless-owner/src/owner_documents.rs` import_document_markdown (no web caller)

## Suggested direction

Add "Import spreadsheet / document" to New, and a convert action in the viewer.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
