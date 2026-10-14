# UXG-023 — Wire the existing "pass to owner" endpoint for members

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-018 |

## Problem

`POST handoffs/{id}/escalate` (`pass_handoff_to_owner`) exists for team leads but no UI calls it. A member with a judgement they should not make can only answer or decline.

## Evidence

- `crates/restless-owner/src/owner.rs:1525-1528`
- `crates/restless-owner/src/owner_teams.rs:124-131`
- no `escalate` call in web/src

## Suggested direction

Add "Pass to owner" to the member's item detail.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
