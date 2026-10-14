# UXG-194 — Put Exec's unread reply and working state on the Exec button, not the People badge

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | ux | US-104 |

## Problem

With the rail closed, the topbar Exec button only shows whether the model route is live; it never says Exec replied or is working. An unread Exec reply instead increments the People tab badge and its desktop notification opens People, pulling the owner out of the rail-centred experience. Owners who keep the rail closed miss replies or land in the wrong surface.

## Evidence

- `web/src/lib/components/AppShell.svelte:408-421` (tb-exec: live/unavailable classes only)
- `web/src/routes/[companyId]/+layout.svelte:763-782` (Exec direct chat counted in unreadChats)
- `web/src/routes/[companyId]/+layout.svelte:797-803` (notification goes to People href)
- `web/src/routes/[companyId]/+layout.svelte:809-829` (badge assigned to People tab)

## Suggested direction

Give the Exec button an unread dot and a working shimmer, and route Exec notifications to open the rail.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
