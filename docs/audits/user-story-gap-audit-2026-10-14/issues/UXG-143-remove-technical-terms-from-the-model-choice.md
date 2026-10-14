# UXG-143 — Remove technical terms from the model choice

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | ux | US-082 |

## Problem

Models shows "Harness" with an OMP tooltip, "restart pending" suffixes, rewritten raw ids; Advanced shows builds and "ACP command (JSON array)". A non-technical owner cannot choose with confidence.

## Evidence

- `web/src/lib/components/AgentIntelligence.svelte:240-262`
- `web/src/lib/components/HarnessDiagnostics.svelte:15-29`
- `web/src/lib/components/CustomHarnesses.svelte:240-275`

## Suggested direction

Lead with quality/cost/speed; hide harness/build behind details.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
