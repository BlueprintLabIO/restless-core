# UXG-097 — Surface or delete the document backlinks API

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | spec-drift | — (found while tracing adjacent paths) |

## Problem

`GET .../documents/{id}/links` ("what links here") and `documents-admin` have no web caller.

## Evidence

- `crates/restless-owner/src/owner_documents.rs:1249-1270`

## Suggested direction

Show backlinks in the inspector, or remove the endpoints.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
