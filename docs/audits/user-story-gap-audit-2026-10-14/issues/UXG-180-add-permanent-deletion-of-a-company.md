# UXG-180 — Add permanent deletion of a company

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-097 |

## Problem

There is no delete route (only archive/restore) and no UI; with one-click creation, mistaken companies pile up in "Show archived" forever.

## Evidence

- `crates/restless-owner/src/owner.rs:1400-1401`
- `web/src/routes/+page.svelte:185-193`

## Suggested direction

"Delete permanently" for archived companies behind typed-name confirmation and an export offer.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
