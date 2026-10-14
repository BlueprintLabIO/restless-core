# UXG-175 — Let Activity filter by date, person and text, and group by day

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-095 |

## Problem

Activity is one flat list with relative times and All/Decisions/External; "what happened yesterday" means scrolling and hovering.

## Evidence

- `web/src/routes/[companyId]/company/activity/+page.svelte:189-198,` :240-305

## Suggested direction

Day headings plus date/actor filters and search.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
