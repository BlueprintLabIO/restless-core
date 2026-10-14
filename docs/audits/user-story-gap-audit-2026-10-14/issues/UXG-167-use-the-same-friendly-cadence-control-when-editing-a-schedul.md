# UXG-167 — Use the same friendly cadence control when editing a schedule

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-091 |

## Problem

Create uses a friendly select; edit switches to free text "30m, 2h or 1d" prefilled in minutes ("1440m"). The list calls paused schedules "Active".

## Evidence

- `web/src/routes/[companyId]/company/schedules/+page.svelte:239,` :280, :339

## Suggested direction

Reuse the create select; split Active and Paused.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
