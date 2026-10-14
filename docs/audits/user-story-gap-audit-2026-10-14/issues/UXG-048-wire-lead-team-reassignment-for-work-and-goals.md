# UXG-048 — Wire lead/team reassignment for Work and Goals

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | missing-function | US-033 |

## Problem

`POST /goals/{goal}/team` exists but no UI calls it; OrgIntel `reassign_work` has no owner route. "Lead" and "Staff" are plain text, so an overloaded lead cannot be relieved.

## Evidence

- `crates/restless-owner/src/owner.rs:1348-1349`
- `crates/restless-owner/src/owner_teams.rs:103`
- `crates/restless-orgintel/src/goals_work.rs:1111`
- `web/src/routes/[companyId]/work/[workId]/+page.svelte:757-762`

## Suggested direction

Make Lead and Goal team pickers backed by these routes, with a reason.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
