# UXG-069 — Add folders or collections to the Library

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | high | missing-function | US-040 |

## Problem

There is no folder, path or move concept in the Library model or owner API. An owner with dozens of client docs, decks and uploads has one flat list filtered only by kind.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:276-277`
- `crates/restless-owner/src/owner_documents.rs:46-193` (no move/parent route)

## Suggested direction

Introduce a lightweight folder/collection grouping that docs, sheets and files can be moved into.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
