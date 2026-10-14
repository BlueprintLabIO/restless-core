# UXG-038 — Show non-owners only the Inbox actions they may take

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | consistency | — (found while tracing adjacent paths) |

## Problem

The list hides "Not needed" from non-owners but the detail always shows "Dismiss", which resolves against the owner projection and likely fails for members (not verified live).

## Evidence

- `web/src/lib/components/InboxList.svelte:353`
- `web/src/lib/components/InboxDetail.svelte:375-382`
- `crates/restless-owner/src/owner.rs:5701-5738`

## Suggested direction

Pass canSetAside into InboxDetail and hide Dismiss for members.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
