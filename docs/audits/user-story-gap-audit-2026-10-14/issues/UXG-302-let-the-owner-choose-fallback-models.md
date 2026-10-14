# UXG-302 — Let the owner choose fallback models

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | low | missing-function | US-189 |

## Problem

Company config holds an ordered `model_failover` list that the CLI sets and copy-setup carries between companies. The owner provider and setup routes accept only a single model, so the web cannot view or edit fallbacks. An owner whose main model hits quota or an outage cannot tell from the cockpit whether the company will keep working, or arrange for it to.

## Evidence

- `crates/restless/src/main.rs:1989-1992` (Set keys include model_failover)
- `crates/restless-owner/src/owner.rs:3594-3602` (CompanyProviderInput has a single model)
- `crates/restless-owner/src/owner.rs:5249-5254` (CompanySetupInput has a single model)
- `crates/restless-owner/src/owner.rs:5010-5028` (copy-setup copies model_failover, the only web path touching it)

## Suggested direction

Show fallback models under the main model on the Model page with add, remove and reorder, written through the provider route.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
