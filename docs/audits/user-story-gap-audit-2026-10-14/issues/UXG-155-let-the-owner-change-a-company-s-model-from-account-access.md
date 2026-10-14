# UXG-155 — Let the owner change a company's model from Account access

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | ux | — (found while tracing adjacent paths) |

## Problem

In Account → Connections → Manage access, a company with access shows only "Has access" and "Remove"; changing its model requires Company → Intelligence, and Remove is refused while in use.

## Evidence

- `web/src/lib/components/AccountConnections.svelte:250-251,` :635-660

## Suggested direction

Show the current model with a Change link or inline select.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
