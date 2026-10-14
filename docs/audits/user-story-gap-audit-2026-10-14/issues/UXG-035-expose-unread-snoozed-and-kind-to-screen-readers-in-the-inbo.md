# UXG-035 — Expose unread, snoozed and kind to screen readers in the Inbox

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | a11y | — (found while tracing adjacent paths) |

## Problem

Unread rows differ only by weight/colour, snoozed rows only by dimming. The link's accessible name is just the title, the kind icon is outside it, and the list is not marked up as a list.

## Evidence

- `web/src/lib/ui/page/Item.svelte:41-53,` :132
- `web/src/lib/components/InboxList.svelte:320-333`

## Suggested direction

Put unread/snoozed/kind in sr-only link text; add list roles.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
