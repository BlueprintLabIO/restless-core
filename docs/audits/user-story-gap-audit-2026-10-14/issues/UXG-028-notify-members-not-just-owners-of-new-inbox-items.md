# UXG-028 — Notify members, not just owners, of new Inbox items

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-021 |

## Problem

Desktop notifications for new Inbox items return early unless the viewer is the owner, so members never hear about handoffs assigned to them.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:914`

## Suggested direction

Run the same notifier over the member's handoff projection.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
