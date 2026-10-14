# UXG-314 — Add an away mode that pauses recurring work and names a deputy

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | missing-function | US-202 |

## Problem

Going on holiday means pausing each schedule one by one. No one but the owner can approve, and notification quieting is left to the operating system's Focus mode. There is no single "I'm away until …" that pauses or keeps chosen schedules, routes decisions to an admin, and narrows pings to blockers. Distinct from UXG-020 (per-item delegation) and UXG-185 (freezing outside actions): this is the combined state of an absent owner.

## Evidence

- `web/src/routes/[companyId]/company/schedules/+page.svelte:270-274` (pause/resume per schedule only)
- `web/src/lib/model/desktop-notify.svelte.ts:1-5` (quiet hours deliberately left to OS Focus)
- `web/src/lib/model/company-access.ts:45-48` (approvals stay with the owner)
- `crates/restless-owner/src/owner.rs:1579-1581` (approval routes)

## Suggested direction

Add one away setting with an end date that pauses selected schedules, names a deputy for approvals within limits, and sends only blockers to Telegram until the owner is back.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
