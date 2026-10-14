# UXG-162 — Offer a non-technical charter editor with a preview

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-087 |

## Problem

Editing the charter opens a raw monospace Markdown textarea; syntax help is a `title` tooltip; no preview.

## Evidence

- `web/src/routes/[companyId]/company/charter/+page.svelte:236-245,` ~:374 (.charter-editor mono)

## Suggested direction

Use the document editor or a rich-text field, or at least a preview tab.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
