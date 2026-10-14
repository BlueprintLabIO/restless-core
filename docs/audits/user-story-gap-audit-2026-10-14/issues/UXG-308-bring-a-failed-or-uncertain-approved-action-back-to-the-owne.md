# UXG-308 — Bring a failed or uncertain approved action back to the owner

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | high | missing-function | US-200 |

## Problem

After the owner allows a call, its result appears only as a decision-history line in Company → Activity ("It ran and failed.") or as an external action marked failed or unknown. Nothing re-enters the Inbox, the badge or Telegram, so an approved email, submission or payment that failed at night goes unnoticed. That gap matters most for "unknown" outcomes, which block retries until someone reconciles them.

## Evidence

- `crates/restless-engine/src/attention.rs:2120-2129` (current_state "It ran and failed." only in decision history)
- `web/src/routes/[companyId]/company/activity/+page.svelte:1-5` (Activity is the only timeline for external effects)
- `web/src/routes/[companyId]/company/activity/+page.svelte:167-173` (failed/unknown shown only as a tone)
- `crates/restless-engine/src/telegram.rs:9-11` (only Attention reaches Telegram)

## Suggested direction

Project a failed or unknown receipt for an owner-approved call as an Attention item with retry, reconcile or drop options and a link to the originating Work.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
