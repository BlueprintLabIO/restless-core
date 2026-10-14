# UXG-132 — Allow editing a connection's address, token or sign-in without removing it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | missing-function | US-073 |

## Problem

There is no PUT/PATCH on `tool-connections/{name}`. Rotating a token means Remove (revoking all permission choices) and re-adding.

## Evidence

- `crates/restless-owner/src/owner.rs:1530-1570`
- `web/src/lib/model/connections.ts:153-189`

## Suggested direction

Add "Replace secret" / "Sign in again" on a working connection that keeps its grants.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
