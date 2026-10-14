# UXG-298 — Break model spend down by agent and model

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | missing-function | US-185 |

## Problem

The spend projection already returns per-actor, per-model spend (`by_actor`), and `restless spend` prints it. The cockpit's Spend section shows one total against the ceiling. Distinct from UXG-049 (cost per Work and Goal) and UXG-169 (periods and categories). This asks which role and which model consumed the budget, which is the first question when the bar jumps.

## Evidence

- `crates/restless-engine/src/coordination.rs:1887-1918` (spend returns by_actor rows of actor, model, spent_usd)
- `crates/restless/src/main.rs:294-296` (CLI Spend: "broken down by actor and model")
- `crates/restless/src/main.rs:3377` (dispatch)
- `web/src/lib/components/CompanyLimits.svelte:194-240` (single total and bar only)

## Suggested direction

Add a fold under the spend bar listing agents and models with their share. Reveal it on request, per the outcomes-first rule.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
