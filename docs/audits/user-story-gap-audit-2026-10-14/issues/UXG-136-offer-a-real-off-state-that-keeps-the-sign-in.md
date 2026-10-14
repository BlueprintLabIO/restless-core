# UXG-136 — Offer a real "off" state that keeps the sign-in

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | missing-function | US-076 |

## Problem

Only Freeze (writes refused, reads continue) or Remove (sign-in deleted). `revokeGrant` exists but is only used inside Remove; grant expiry is never offered.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:247-262,` :466
- `web/src/lib/model/connections.ts:24-31,` :177-178

## Suggested direction

Add "Off" (revoke grant, keep sign-in), optionally with expiry.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
