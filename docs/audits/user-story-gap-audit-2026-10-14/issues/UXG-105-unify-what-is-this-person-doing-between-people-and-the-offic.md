# UXG-105 — Unify "what is this person doing" between People and the office floor

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | consistency | US-061 |

## Problem

The pixel office shows presence, observation time, current step and Open Work; People shows less.

## Evidence

- `web/src/lib/office/OfficeCanvas.svelte:1784-1820`
- `web/src/routes/[companyId]/people/+page.svelte:176-213`

## Suggested direction

Feed the same projection (lib/office/projection.ts) into People.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
