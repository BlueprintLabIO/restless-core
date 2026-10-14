# UXG-159 — Make the empty-portfolio text match the real button

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | consistency | US-085 |

## Problem

The empty state says "Use New company"; the only button is "Start a company".

## Evidence

- `web/src/routes/+page.svelte:231-233`
- `web/src/lib/components/CreateCompany.svelte:81`

## Suggested direction

Fix the copy or put the action in the empty state.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
