# UXG-095 — Decide whether archived documents stay editable

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | consistency | — (found while tracing adjacent paths) |

## Problem

Archived docs (and sheets) open fully editable and the server issues write tokens regardless of status, while the banner says "Archived".

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:154-156`
- `crates/restless-owner/src/owner_documents.rs:761-768`

## Suggested direction

Make archived items read-only until restored, or say on the banner that edits continue.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
