# UXG-094 — Show agent document proposals as a diff, not two full texts

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | ux | — (found while tracing adjacent paths) |

## Problem

Accepting an agent proposal shows the whole current and proposed text side by side in `<pre>` blocks; on a long doc the owner cannot see what changed. A word diff renderer already exists.

## Evidence

- `web/src/lib/components/DocumentInspector.svelte:1663-1672`

## Suggested direction

Reuse the version-diff renderer for proposals.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
