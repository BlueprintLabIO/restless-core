# T7 — Dogfood the Three-Harness Release and Purge Scaffolding

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** End-to-End Verification + Release  
**Depends on:** T2, T3, T4, T5, T6

## Outcome

Restless has live evidence that its default, Codex, and Claude Agent harnesses produce usable owner alignment and productive work without weakening policy, attribution, recovery, or cleanup. Each harness receives an explicit release disposition.

## Work

- Run one frozen semantic corpus against Restless Managed, Codex, and Claude Agent using clean company environments.
- Include a hard-goal Attention scenario with a deliberately broad search space. Verify that the owner can narrow it through the existing rail and then complete an explicit typed decision.
- Include a productive repository edit with tools, an approved external/MCP path, permission refusal, cancellation during activity, adapter/process failure, and runtime restart/reconstruction.
- Inspect process trees, config/storage roots, environment/argv/logs, session locators, canonical messages, usage records, and cleanup artifacts.
- Run cross-company and cross-actor isolation cases concurrently.
- Compare result usability and latency as observations only; do not change the default automatically.
- Have a fresh reviewer reproduce the critical path from the release build and verify evidence against the sprint acceptance criteria.
- Record `supported`, `supported with named optional limitations`, or `not shipped` independently for each harness.
- Purge prototype scripts, permissive flags, raw-key routes, duplicated adapters, stale `worker_runtime` product copy, and any custom-agent/picker scaffolding.

## Acceptance

- The owner completes the hard-goal work-through flow on all harnesses marked supported, and the item remains unresolved until the explicit typed action.
- Each supported harness completes the productive edit, connected-tool, permission, cancellation, failure, restart, and cleanup cases.
- Evidence shows no provider secret in Runtime, no ambient config augmentation, no cross-company state, no duplicate canonical events, and no orphan process.
- Existing companies on Restless Managed show no behavioral or configuration regression.
- Result and latency observations are recorded with inputs and limitations; no synthetic score is presented as a product guarantee.
- A fresh reviewer signs the acceptance matrix from a release-like image.
- The release notes name the precise support level and optional capability gaps for each harness.
- No exploratory scaffolding remains in production paths.

## Makes Deletable

- The manual three-harness dogfood checklist.
- Prototype launch and credential scripts.
- Temporary compatibility switches and permissive fallbacks.
- Claims of “ACP support” based only on handshake or toy prompt completion.

## Stop

Do not mark Claude Agent or Codex supported if any required semantic case fails. Ship the passing harnesses and state the missing support plainly rather than weakening the common acceptance boundary.
