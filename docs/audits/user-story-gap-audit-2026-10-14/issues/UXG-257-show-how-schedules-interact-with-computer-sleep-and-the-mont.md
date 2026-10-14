# UXG-257 — Show how schedules interact with computer sleep and the monthly hours limit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-149 |

## Problem

Each due schedule wakes a sleeping computer, so a 30-minute schedule can keep it from ever reaching its sleep timeout, and once the monthly hours limit is reached scheduled wakes simply fail. The Schedules page explains intervals and time zones but says nothing about computer time, and the Limits page describes waking for "due schedules" without linking to them. Owners set cadence and runtime limits in two places with no view of their combined effect. Distinct from UXG-165 and UXG-167, which concern cadence choices.

## Evidence

- `web/src/routes/[companyId]/company/schedules/+page.svelte:200-203` (Repeat help text)
- `crates/restless-engine/src/schedule.rs:1436-1438` (due work wakes the computer)
- `crates/restless-engine/src/runtime_sleep.rs:206-216` (cap check in the sleep loop)
- `web/src/lib/components/CompanyLimits.svelte:420-424`

## Suggested direction

Show the expected awake time a cadence implies and a warning on the schedule when the hours limit has stopped its runs.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
