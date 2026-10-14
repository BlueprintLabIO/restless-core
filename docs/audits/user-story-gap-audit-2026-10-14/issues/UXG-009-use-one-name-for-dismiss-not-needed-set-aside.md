# UXG-009 — Use one name for "Dismiss / Not needed / Set aside"

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | consistency | US-012 |

## Problem

The same action is "Dismiss" in the detail header, "Not needed" in the row menu, and "Set aside" in the toast and fold. Owners cannot tell they are one action or where the item went.

## Evidence

- `web/src/lib/components/InboxDetail.svelte:382`
- `web/src/lib/components/InboxList.svelte:111,` :353-356
- `web/src/lib/components/SetAside.svelte:61-62`

## Suggested direction

Pick one verb and use it everywhere.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
