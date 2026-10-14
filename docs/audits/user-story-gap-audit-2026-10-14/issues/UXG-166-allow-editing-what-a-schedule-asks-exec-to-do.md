# UXG-166 — Allow editing what a schedule asks Exec to do

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | high | missing-function | US-091 |

## Problem

After creation only cadence/time can change; `ScheduleUpdateInput` has no prompt field. Fixing a wording mistake means deleting the schedule and losing its history.

## Evidence

- `web/src/lib/model/skills.ts:54-71`
- `crates/restless-owner/src/owner_skills.rs:618-628`
- `web/src/routes/[companyId]/company/schedules/+page.svelte:268-304`

## Suggested direction

Accept prompt in schedule updates and add Edit.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
