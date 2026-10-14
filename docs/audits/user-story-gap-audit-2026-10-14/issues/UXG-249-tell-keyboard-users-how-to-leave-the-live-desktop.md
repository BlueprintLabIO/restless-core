# UXG-249 — Tell keyboard users how to leave the live desktop

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | a11y | US-143 |

## Problem

Once focus is inside the desktop canvas every key, including Tab, goes to the remote computer. The only escape is Shift+Escape, which moves focus to the toolbar, but no label, tooltip or help text mentions it, and the shortcuts sheet does not list it. A keyboard or screen-reader user is effectively trapped and cannot reach Release control or Leave computer. Distinct from UXG-031, which concerns keys the shortcuts sheet omits in general.

## Evidence

- `web/src/lib/components/DesktopViewport.svelte:268-273` (Shift+Escape handler)
- `web/src/lib/components/DesktopViewport.svelte:637-651` (no visible hint in the desktop tools or status)

## Suggested direction

Announce the escape key when the desktop takes focus and show it in the desktop toolbar and the shortcuts sheet.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
