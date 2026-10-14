# UXG-158 — Use a multi-line answer box on the welcome page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | ux | US-085 |

## Problem

The owner describes the business in a single-line input; long answers scroll sideways on phones.

## Evidence

- `web/src/routes/[companyId]/welcome/+page.svelte:271-279`

## Suggested direction

Auto-growing textarea, Enter to send, Shift+Enter for newline.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
