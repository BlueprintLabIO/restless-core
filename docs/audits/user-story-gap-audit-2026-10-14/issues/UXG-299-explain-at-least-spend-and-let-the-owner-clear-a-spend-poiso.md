# UXG-299 — Explain "At least" spend and let the owner clear a spend poison

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | ux | US-186 |

## Problem

When some model responses report no price, spend enters metering_unknown and the cockpit quietly prefixes the total with "At least". The engine has an explanatory note and an owner-only `clear-poison` (plus an audited `spend-correct` for duplicates), but the web shows neither the reason nor any action. The owner cannot tell whether the limit still protects them or how to restore exact accounting.

## Evidence

- `web/src/lib/components/CompanyLimits.svelte:222-223` ("At least" prefix with no explanation)
- `crates/restless-engine/src/coordination.rs:1913-1917` (note explaining the lower bound)
- `crates/restless-engine/src/coordination.rs:4214-4219` (clear-poison resumes exact accounting)
- `crates/restless-engine/src/wire.rs:1292` (clear-poison is OWNER_ONLY)
- `crates/restless/src/main.rs:404-409` (CLI ClearPoison)
- `crates/restless/src/main.rs:305-325` (CLI SpendCorrect)

## Suggested direction

Show the note as an inline tooltip on "At least", and offer a confirmed "Resume exact accounting" action once the owner has seen which calls were unpriced.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
