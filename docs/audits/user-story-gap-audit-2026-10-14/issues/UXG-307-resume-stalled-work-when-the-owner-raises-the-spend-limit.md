# UXG-307 — Resume stalled work when the owner raises the spend limit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | missing-function | US-199 |

## Problem

Raising the spend limit saves the new limit and records it, then returns the company projection. It sends no wake to the Exec or to the Work that stopped on "[budget]". The owner fixes the cause but has no sign that anything restarted, and has to nudge Exec in chat to find out. This breaks the rule that the owner should never have to report that a step is done when the system can see it.

## Evidence

- `crates/restless-owner/src/owner_company_settings.rs:103-143` (limit saved and recorded
- no wake or resume)
- `crates/restless-engine/src/staff/execution.rs:188-196` (Work attempt ends Blocked with budget summary)
- `web/src/lib/components/CompanyLimits.svelte:200` ("total model budget… not a monthly one")

## Suggested direction

When the ceiling rises above accounted spend, treat that as the observable resume condition and wake the actors whose last turn was blocked on budget.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
