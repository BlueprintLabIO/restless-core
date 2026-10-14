# UXG-021 — Let a requester see all their requests and withdraw a pending one

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-017 |

## Problem

"Your requests" shows at most 3 rows, only in the Exec rail. Rows are not clickable or cancellable and there is no withdraw endpoint. The spec puts this list in For you.

## Evidence

- `web/src/lib/components/MyRequests.svelte:21-29,` :46-55
- `web/src/lib/components/ExecutiveRail.svelte:1342`
- `docs/specs/direction-and-permission.md` §2

## Suggested direction

Add a full list with withdraw and a For you section.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
