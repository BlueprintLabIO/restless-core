# UXG-309 — Surface failed and skipped scheduled runs outside the Schedules page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | ux | US-201 |

## Problem

The Attention projection deliberately leaves scheduled-run failures on the Schedules page, and the Company tab only gets a badge for a provider issue. A weekly report or daily check that failed, or that was skipped because it was missed by more than two hours, is visible only to an owner who opens Company → Schedules. Distinct from UXG-173 (Health next steps) and UXG-166 (editing schedules): this is about how the owner finds out at all.

## Evidence

- `crates/restless-engine/src/attention.rs:1472-1473` ("Scheduled opportunity failures are shown in Schedules")
- `web/src/routes/[companyId]/+layout.svelte:820-828` (Company tab badge only for providerIssue)
- `web/src/routes/[companyId]/company/schedules/+page.svelte:202` (missed runs over two hours are skipped)
- `web/src/routes/[companyId]/company/schedules/+page.svelte:257-258` ("Last run needs attention" only here)

## Suggested direction

Give the Company tab a dot for a schedule whose last run failed or was skipped, and let Exec raise an Inbox item only when a rerun needs the owner.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
