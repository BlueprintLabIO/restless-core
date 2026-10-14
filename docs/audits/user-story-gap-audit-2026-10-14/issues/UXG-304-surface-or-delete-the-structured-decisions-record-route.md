# UXG-304 — Surface or delete the structured decisions record route

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | low | spec-drift | US-193 |

## Problem

GET /decisions serves a read-only record of Exec's structured decisions, with probabilities and observed outcomes, summarising how often Exec wakes for nothing. No web code calls it. The old Company → Decisions route now redirects to Activity, which reads attention history instead. The endpoint is either a missing owner insight into Exec's calibration or dead surface area.

## Evidence

- `crates/restless-owner/src/owner.rs:1626` (decisions route)
- `crates/restless-owner/src/owner_decisions.rs:1-4` (what the record answers)
- `web/src/routes/[companyId]/company/decisions/+page.ts:3-5` (redirects to Activity)
- `web/src/routes/[companyId]/company/activity/+page.svelte:50` (Activity merges attention history decisions, not this record)

## Suggested direction

Decide in the next sprint whether calibration belongs in Company → Activity, revealed on request. If not, remove the route.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
