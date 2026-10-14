# UXG-276 — Stop offering "Add <app>" for apps already connected and let ⌘K find what the company has

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | ux | US-159 |

## Problem

⌘K lists "Add Slack", "Add GitHub" etc. for every catalogue entry even when that app is already connected, and offers no row for the connected app itself, a schedule, a piece of know-how or an Activity entry. Owners typing an app or schedule name land on an add flow or nothing. Members also cannot find their own handoffs, because "Needs you" rows are owner-only.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:1081-1088` (Add row for every CATALOGUE entry)
- `web/src/routes/[companyId]/+layout.svelte:961-1062` (shared commands: no apps, schedules, know-how or activity)
- `web/src/routes/[companyId]/+layout.svelte:1101-1107` (Needs you rows owner-only)

## Suggested direction

Build app rows from the company’s connections (Open vs Add) and add schedules and know-how to the command list.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
