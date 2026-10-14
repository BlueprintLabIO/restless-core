# UXG-077 — Show the actual old document when previewing history

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | ux | US-044 |

## Problem

Preview renders only a list of word-level changes, never the page as it was, so the owner cannot read the old version before restoring.

## Evidence

- `web/src/lib/components/DocumentInspector.svelte:1750-1830`

## Suggested direction

Render the version's saved body with an option to show changes.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
