# UXG-050 — Add goal editing, close-this-goal and reopen in the Work UI

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | high | missing-function | US-035 |

## Problem

Goals are created only via `/goal <objective>` (no finish line or due date) and closed only via `/goal clear`, which closes the latest open goal regardless of selection. `set_goal_details` exists in OrgIntel with no owner route; there is no reopen.

## Evidence

- `web/src/lib/model/skills.ts:307-326`
- `crates/restless-owner/src/owner_skills.rs:178-180,` :222
- `crates/restless-orgintel/src/goals_work.rs:106`
- `web/src/routes/[companyId]/work/+page.svelte:539-564`

## Suggested direction

Add a goal menu: rename, finish line, due date, close (confirm), reopen.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
