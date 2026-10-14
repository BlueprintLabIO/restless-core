# UXG-260 — Offer a fix when installed agent software does not match the required build

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | low | ux | US-151 |

## Problem

"Installed agent software" lists each harness's installed build beside the required one and a status such as "Configuration required", but offers no action. The owner learns the computer runs the wrong Claude Code or Codex build and has nowhere to go, although the engine already detects image drift that a rebuild fixes.

## Evidence

- `web/src/lib/components/HarnessDiagnostics.svelte:8-10` ('incompatible' shown as Configuration required)
- `web/src/lib/components/HarnessDiagnostics.svelte:22-27` (installed vs required, no action)
- `crates/restless-engine/src/company.rs:1742-1749` (runtime_image_drifted)

## Suggested direction

Link a mismatch to the rebuild action on Health, or explain who updates it and when.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
