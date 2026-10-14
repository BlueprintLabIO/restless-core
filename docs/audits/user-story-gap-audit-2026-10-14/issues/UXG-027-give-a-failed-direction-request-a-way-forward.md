# UXG-027 — Give a failed direction request a way forward

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | spec-drift | US-016 |

## Problem

The spec says an approval can never be "approved but not done", yet the UI shows a `failed` state ("Approved, not done") with no retry. The item already left the Inbox, so nobody owns fixing it.

## Evidence

- `web/src/lib/components/DirectionRequestDetail.svelte:94-95,` :124
- `web/src/lib/components/MyRequests.svelte:40-41`
- `docs/specs/direction-and-permission.md:94`

## Suggested direction

Bring a failed request back to the approver with Retry or Ask Exec.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
