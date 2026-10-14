# UXG-082 — Let an old document version be downloaded

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | consistency | US-044 |

## Problem

`GET .../versions/{version}/export/markdown` exists with no web caller; sheets can download any version.

## Evidence

- `crates/restless-owner/src/owner_documents.rs` export_document_markdown

## Suggested direction

Add "Download this version" to History preview.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
