# UXG-042 — Separate "pause" from "stop for good" and warn before abandoning

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | high | ux | US-029 |

## Problem

"Stop…" sits beside "Resume…" and reads like a pause, but calls `/abandon` ("Stop Work for good"). The form never says it is permanent, the toast just says "Stopped", and no pause exists.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:129,` :446-450, :484-496
- `crates/restless-owner/src/owner_teams.rs:173`

## Suggested direction

Rename to "Stop for good" with irreversibility copy, and add a pause that Resume can undo.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
