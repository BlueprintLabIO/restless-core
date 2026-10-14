# UXG-267 — Make Undo reachable from the keyboard within its window

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | a11y | US-166, US-168 |

## Problem

The toast region renders after the whole page and rail, focus is never moved to it and there is no shortcut, so a keyboard user must Tab through the entire workspace to reach Undo inside 10 s. The hold-on-focus logic only helps once focus is already there. In practice Undo is a mouse-only feature, although the confirm policy relies on Undo instead of asking first for reversible acts.

## Evidence

- `web/src/routes/+layout.svelte:245-249` (Toasts mounted after all page content)
- `web/src/lib/ui/feedback/Toasts.svelte:12-35` (no focus move, no key binding)
- `web/src/lib/ui/feedback/confirm.svelte.ts:1-3` (reversible acts rely on Undo instead of confirming)
- `web/src/lib/components/ShortcutsDialog.svelte:143-193` (no undo shortcut listed)

## Suggested direction

Add a global Undo shortcut (e.g. ⌘Z outside editors) for the newest undoable toast and announce it in the toast text.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
