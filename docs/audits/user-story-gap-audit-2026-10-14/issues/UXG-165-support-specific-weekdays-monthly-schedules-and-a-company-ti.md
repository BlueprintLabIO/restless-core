# UXG-165 — Support specific weekdays, monthly schedules and a company time zone

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-090 |

## Problem

Schedules offer fixed intervals or "Every weekday at…": no single weekday, no monthly, no time-zone choice (taken silently from the creating browser), and "Every day" fires at creation time, not a chosen hour.

## Evidence

- `web/src/routes/[companyId]/company/schedules/+page.svelte:51,` :209-221
- `web/src/lib/model/skills.ts:279-285`

## Suggested direction

Add day-of-week/day-of-month options and a changeable company time zone.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
