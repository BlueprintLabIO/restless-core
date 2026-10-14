# UXG-153 — Let custom harnesses be edited and removed; confirm harness disconnect

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | missing-function | — (found while tracing adjacent paths) |

## Problem

Custom harness actions are install/probe/setup/api_key only; no edit or uninstall. Company-only sign-in "Disconnect" has no confirmation.

## Evidence

- `crates/restless-owner/src/owner_custom_harnesses.rs:133-146`
- `web/src/lib/components/CustomHarnesses.svelte:287-321`
- `web/src/lib/components/HarnessConnections.svelte:169-173`

## Suggested direction

Add Remove/Edit; confirm disconnects of an in-use sign-in.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
