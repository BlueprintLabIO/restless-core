# UXG-305 — Prune unused client contracts left after the route parity check

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | low | consistency | US-194 |

## Problem

A route-by-route check of every fetch path in web/src against owner.rs found no web call to a missing route and no request-shape mismatch in the spot-checked writes (approvals, team owner, Work resume and abandon, standards, schedules, For-you, library archive, copy-setup, browser control). What remains are client contracts with no caller: `CreateDocumentVersionInput`, `directionApplied` and `searchRoomTitles`. Each reads as a supported capability to the next agent and goes stale silently.

## Evidence

- `web/src/lib/model/documents.ts:329` (CreateDocumentVersionInput unused)
- `web/src/lib/model/direction.ts:57` (directionApplied unused)
- `web/src/lib/model/rooms.ts:357` (room-title search helper with no UI caller)
- `crates/restless-owner/src/owner_teams.rs:7-14` (TeamOwnerInput matches web body, example of a verified shape)
- `crates/restless-owner/src/owner_skills.rs:620-627` (ScheduleUpdateInput matches web/src/lib/model/skills.ts:61-69)

## Suggested direction

When UXG-297, EP18 and the room-search issue are decided, either wire each contract or delete it in the same change.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
