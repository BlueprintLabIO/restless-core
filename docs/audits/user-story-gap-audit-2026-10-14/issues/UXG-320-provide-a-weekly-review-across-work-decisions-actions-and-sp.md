# UXG-320 — Provide a weekly review across work, decisions, actions and spend

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | missing-function | US-198 |

## Problem

No surface answers "what did my company do this week and what did it cost". The digest endpoint accepts a day, but the only consumer fetches today. Activity explicitly leaves out Work outcomes. The Work board has no completion dates or period filter. Spend is a lifetime total with no history. Distinct from UXG-049 (per-Work cost), UXG-169 (budget periods) and UXG-175 (Activity filters), which each cover one piece of the review.

## Evidence

- `web/src/lib/components/DirectionToday.svelte:9-17` (digest fetched for today only)
- `web/src/lib/model/direction.ts:78-79` (digest supports ?day=)
- `web/src/routes/[companyId]/company/activity/+page.svelte:3-5` (Work outcomes excluded)
- `web/src/lib/components/CompanyLimits.svelte:200` (total, "not a monthly one")
- `web/src/routes/[companyId]/work/+page.svelte:217` (Work sorted by updated_at only)

## Suggested direction

Let Exec prepare a weekly review as a native document from the same projections (finished Work, decisions, external actions, spend delta), offered from the Inbox each week.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
