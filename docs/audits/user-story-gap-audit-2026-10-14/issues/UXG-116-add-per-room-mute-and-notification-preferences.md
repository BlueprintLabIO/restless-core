# UXG-116 — Add per-room mute and notification preferences

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | missing-function | US-067 |

## Problem

Nothing implements mute or per-conversation notification settings; desktop notifications are one global toggle.

## Evidence

- `web/src/lib/components/Room*.svelte,` web/src/lib/model/rooms.ts, web/src/lib/model/room-queries.svelte.ts (no mute)
- `web/src/lib/model/desktop-notify.svelte.ts` (global)

## Suggested direction

Add a per-viewer Mute toggle suppressing badges and notifications.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
