# UXG-024 — Gate "Hold to accept" and "Request changes" with the same real roles

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | consistency | — (found while tracing adjacent paths) |

## Problem

In review, Request changes is disabled unless `canDecide`, but Hold to accept is not. `canDecide` checks `['owner','operator']`, leaving out `admin`; "operator" is not a role the shell uses.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:199,` :1016-1023, :1037-1043

## Suggested direction

Gate both the same way using owner/admin/member.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
