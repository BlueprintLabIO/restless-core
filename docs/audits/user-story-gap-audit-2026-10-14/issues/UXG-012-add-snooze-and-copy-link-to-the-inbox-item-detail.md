# UXG-012 — Add Snooze and Copy link to the Inbox item detail

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | mobile | US-011, US-023 |

## Problem

Snooze, Wake and Copy link exist only in the row ⋯ menu and as keys. On phones the detail hides the list, so you cannot snooze the item you are reading.

## Evidence

- `web/src/lib/components/InboxList.svelte:346-357`
- `web/src/lib/components/InboxDetail.svelte:384-399`
- `web/src/routes/[companyId]/+page.svelte:1217-1219`

## Suggested direction

Put snooze and copy link in the detail's ⋯ menu.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
