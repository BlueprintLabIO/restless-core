# UXG-248 — Let a phone user type into the live desktop

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | high | mobile | US-142 |

## Problem

DesktopViewport builds the noVNC RFB client directly and captures keydown on the canvas; its toolbar offers only Clipboard and Fullscreen. Touching the canvas on a phone never raises the soft keyboard, there are no Ctrl, Alt, Tab or Esc keys, and the clipboard panel's instruction to "Press Ctrl+V" cannot be followed without a hardware keyboard. An owner handed a 2FA code entry or a CAPTCHA while away from a desk cannot complete it.

## Evidence

- `web/src/lib/components/DesktopViewport.svelte:203-209` (core rfb.js used without the noVNC UI keyboard)
- `web/src/lib/components/DesktopViewport.svelte:292-300` (key events captured on the canvas only)
- `web/src/lib/components/DesktopViewport.svelte:640-671` (toolbar has clipboard and fullscreen only)
- `web/src/lib/components/DesktopViewport.svelte:585` (Ctrl+V instruction)

## Suggested direction

Add a keyboard button that focuses a hidden input forwarding keys, plus a small extra-keys row, shown on coarse pointers.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
