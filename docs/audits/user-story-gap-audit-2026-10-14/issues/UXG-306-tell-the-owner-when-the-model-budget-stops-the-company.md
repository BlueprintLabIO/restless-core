# UXG-306 — Tell the owner when the model budget stops the company

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | high | missing-function | US-199 |

## Problem

When known model spend reaches the ceiling, Exec and every Staff turn stop with a "[budget]" block, but the result never becomes an Inbox item. The only start-up blocker projected to Attention covers credential, quota or model failures. The overdue-dispatch item checks only a missing model route or a stopped computer, the Inbox "start blocker" reads only the catalogue's unstartable reason, and Telegram forwards only Attention. An owner whose company stops at 2am finds out only by noticing silence, then digging into Company → Limits. Distinct from UXG-169 (periods and early warnings) because this is about the hard stop itself never reaching any channel.

## Evidence

- `crates/restless-engine/src/exec.rs:315-321` (Exec turn refused with [budget] reason)
- `crates/restless-engine/src/staff/execution.rs:182-197` (Staff turn returns Blocked on budget)
- `crates/restless-engine/src/attention.rs:2384-2401` (exec_startup_failure: Budget kind falls to `_ => None`, so not projected)
- `crates/restless-engine/src/attention.rs:1620-1628` (dispatch_blocked ignores budget)
- `web/src/routes/[companyId]/+page.svelte:139-145` (startBlocker uses unstartable_reason only)
- `crates/restless-engine/src/telegram.rs:9-11` (Attention is the only thing sent)

## Suggested direction

Project an exhausted budget as one source-authored Attention blocker linking to Limits, so it also reaches Telegram and the Inbox badge.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
