# UXG-087 — Fix Library table head and sort accessibility

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | a11y | US-043 |

## Problem

The "Owner" header is aria-hidden and display:none (dead markup) while its slot holds owner/size/excerpt; sort buttons expose state only via `title`, with no aria-sort.

## Evidence

- `web/src/routes/[companyId]/library/+page.svelte:1190-1221,` :1206, :1539-1541

## Suggested direction

Use a proper table/grid with aria-sort; remove or relabel the hidden column.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
