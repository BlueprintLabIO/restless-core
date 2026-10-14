# UXG-169 — Add budget periods, categories and early warnings

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | high | spec-drift | US-092 |

## Problem

Spend is a single running USD total ("not a monthly one"): no monthly option, no category/period breakdown, no remaining figure, and the only warning is a colour change at 80%. Spec §8.2/§12.5/§14.4 require categories, period, remaining and threshold updates.

## Evidence

- `web/src/lib/components/CompanyLimits.svelte:195-240`
- `web/src/routes/[companyId]/company/+page.svelte:20-27`
- `docs/specs/owner-cockpit.md:917-925,` :1345, :1473

## Suggested direction

Add a period choice and a threshold that raises an Inbox item before the ceiling.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
