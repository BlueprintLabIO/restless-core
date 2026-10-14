# UXG-007 — Let the owner export or share the brand guide

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | low | missing-function | US-005 |

## Problem

There is no export (PDF/Markdown) or share link for the identity, so handing the brand to an outside designer or printer means copy-pasting statements from folds.

## Evidence

- `web/src/routes/[companyId]/company/identity/+page.svelte` (no export/share action anywhere on the page)

## Suggested direction

Follows from UXG-002: once identity is a document, reuse document export and sharing.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
