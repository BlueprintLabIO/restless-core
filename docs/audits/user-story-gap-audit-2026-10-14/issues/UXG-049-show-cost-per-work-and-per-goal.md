# UXG-049 — Show cost per Work and per Goal

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | spec-drift | US-034 |

## Problem

Spend records carry `work_id`, but the owner API projects spend only per person and company. Spec §6.3/§6.4 ask for cost/time signals on work and goals.

## Evidence

- `crates/restless-model-gateway/src/spend.rs:82`
- `web/src/lib/model/generated/cockpit.ts:18,` :44
- `docs/specs/owner-cockpit.md:662,` :679

## Suggested direction

Aggregate spend by work_id and goal; show it in the detail rail and goal header.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
