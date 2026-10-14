# UXG-020 — Let the owner delegate an Inbox item to a member or admin

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-014 |

## Problem

Spec §5.5 lists "delegate" and "ask a named employee". The detail only offers reply, decline, dismiss or discuss-with-lead.

## Evidence

- `web/src/lib/components/InboxDetail.svelte:384-399,` :545-598
- `docs/specs/owner-cockpit.md` §5.5

## Suggested direction

Add "Hand to…" in ⋯, backed by handoff reassignment.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
