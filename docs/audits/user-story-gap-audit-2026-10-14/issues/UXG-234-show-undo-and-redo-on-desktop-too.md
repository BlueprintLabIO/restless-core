# UXG-234 — Show Undo and Redo on desktop too

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | ux | US-137 |

## Problem

Undo and Redo buttons exist only for touch screens; on a laptop they are reachable only by keyboard shortcut. A non-technical owner who deletes a paragraph by accident, or is told to "Undo it" by the save-problem banner, has no visible control unless the banner appears.

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:1036-1053` (Undo / Redo marked touch-only)
- `web/src/lib/components/DocumentEditor.svelte:1490-1497` (.touch-only hidden unless hover: none)

## Suggested direction

Show Undo / Redo in the … menu on every device, each naming its shortcut in a tooltip.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
