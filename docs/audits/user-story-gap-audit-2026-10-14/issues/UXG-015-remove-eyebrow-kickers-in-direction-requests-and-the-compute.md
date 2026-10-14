# UXG-015 — Remove eyebrow kickers in direction requests and the computer view

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | consistency | US-016 |

## Problem

A small uppercase "Request" kicker sits above the request heading, and the computer view has a "LIVE OUTCOME UNAVAILABLE" mono kicker above an h2. Both repeat context, which the repo's no-eyebrow rule forbids.

## Evidence

- `web/src/lib/components/DirectionRequestDetail.svelte:104,` :196-200
- `web/src/routes/[companyId]/+page.svelte:919`

## Suggested direction

Drop the kickers; use a tooltip if the kind needs saying.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
