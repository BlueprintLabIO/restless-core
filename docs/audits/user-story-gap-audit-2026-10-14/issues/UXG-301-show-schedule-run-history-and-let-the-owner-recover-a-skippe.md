# UXG-301 — Show schedule run history and let the owner recover a skipped run

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | missing-function | US-188 |

## Problem

The schedule help says a weekday run missed by more than two hours is skipped. The cockpit shows only "last fired", so a skipped run disappears silently. The CLI can list fired and skipped occurrences, recover a skipped one with one idempotent wake, retry a failed recovery and set the missed-time policy (skip, skip-if-late, catch-up, coalesce-latest). None of these are in the owner API or the web. Distinct from UXG-165 to UXG-168, which cover cadence editing and Run now.

## Evidence

- `web/src/routes/[companyId]/company/schedules/+page.svelte:202` ("a run missed by more than two hours is skipped")
- `web/src/routes/[companyId]/company/schedules/+page.svelte:355-357` (only last_fired_at shown)
- `crates/restless-owner/src/owner_skills.rs:342-343` (later missed run is skipped)
- `crates/restless/src/main.rs:1456-1550` (List --all / History / Recover / RetryRecovery / Policy)
- `crates/restless/src/main.rs:3850-3910` (dispatch)

## Suggested direction

Add a per-schedule history fold that marks skipped runs with a Recover action, and a "When a run is missed" choice on the schedule row.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
