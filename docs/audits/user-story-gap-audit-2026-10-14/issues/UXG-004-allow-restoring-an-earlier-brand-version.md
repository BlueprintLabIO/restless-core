# UXG-004 — Allow restoring an earlier brand version

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | medium | missing-function | US-006 |

## Problem

Version history lists every release in folds but offers no "Restore this version". Rolling back means retyping old statements by hand, and non-owner statements cannot be retyped at all.

## Evidence

- `web/src/routes/[companyId]/company/identity/+page.svelte:398-415`

## Suggested direction

Add "Restore as new version" on a release, creating a new attributed release from its evidence set.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
