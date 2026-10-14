# UXG-047 — Show attempt history, feedback and inputs on Work detail

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | spec-drift | US-025, US-036 |

## Problem

Only `attempts.at(-1)` is shown; `attempt_feedback`, `attempt_inputs`, `attempt_limit` and `producing_topology` are never read. Spec §12.3 requires revisions, attempts and exact upstream inputs.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:166-171,` :706-727
- `web/src/lib/model/generated/orgintel.ts:201`

## Suggested direction

Add a compact attempt timeline (outcome, feedback, inputs) under Run details.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
