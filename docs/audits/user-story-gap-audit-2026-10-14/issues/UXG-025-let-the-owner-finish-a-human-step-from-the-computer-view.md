# UXG-025 — Let the owner finish a human step from the computer view

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | ux | US-019 |

## Problem

After finishing a sign-in on the company computer the owner must leave, find the item again and press "Mark done". Mark done in the list (and E) has no confirmation or undo.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:806-829`
- `web/src/lib/components/InboxList.svelte:206-222`
- `web/src/lib/components/InboxDetail.svelte:676-682`

## Suggested direction

Show the resume condition and its observed state in the handover bar; add undo to Mark done.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
