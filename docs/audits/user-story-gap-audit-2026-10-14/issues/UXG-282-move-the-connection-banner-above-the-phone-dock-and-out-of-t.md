# UXG-282 — Move the connection banner above the phone dock and out of the toasts’ way

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | mobile | US-156, US-164 |

## Problem

The offline / reconnecting / reconnected banner is fixed 18 px from the bottom with the app-level z-index, so on a phone it sits on top of the bottom dock tabs it is meant to coexist with, and on every screen it occupies the same bottom-centre spot as toasts. Toasts already offset themselves above the dock; the banner does not, so during an outage the owner can lose access to navigation or have the banner and an Undo toast overlap.

## Evidence

- `web/src/routes/+layout.svelte:207-221` (banner markup)
- `web/src/routes/+layout.svelte:280-297` (bottom 18px, z-app, centred)
- `web/src/lib/ui/feedback/Toasts.svelte:38-54` (toasts offset above the dock)
- `web/src/lib/design/chrome.css:372-380` (dock fixed at bottom)
- `web/src/lib/ui/style/base.css:18` (z-app 1000)

## Suggested direction

Give the banner the same dock-aware bottom offset as toasts and stack the two in one region.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
