# UXG-055 — Fix the "No work yet" empty state on a failed search

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | ux | US-030 |

## Problem

A zero-result search with no goal selected shows "No work yet", as if the company had no work, and offers no clear-search action.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:628-630`
- `web/src/lib/model/work-views.ts:3-6`

## Suggested direction

Show "No work matches '…'" with a clear button.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
