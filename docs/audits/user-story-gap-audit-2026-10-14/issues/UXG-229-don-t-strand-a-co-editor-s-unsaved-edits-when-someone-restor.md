# UXG-229 — Don't strand a co-editor's unsaved edits when someone restores a version

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | ux | US-130 |

## Problem

When another person restores an earlier version, every open editor becomes read-only and the owner is told to copy any recent changes by hand before loading the restored version. There is no merge, no list of what would be lost, and no warning to the restorer that colleagues are mid-edit. Distinct from UXG-077/UXG-078 (previewing and restoring history): this is what happens to the people already editing.

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:407-416` ("Copy any recent changes you want to keep from this read-only draft")
- `web/src/lib/components/DocumentEditor.svelte:1165-1172` (Load restored version button)

## Suggested direction

Warn the restorer who is editing, and re-apply the stranded user's unsynced edits on top of the restored version (or save them as a proposal) instead of asking for manual copying.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
