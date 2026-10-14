# UXG-070 — Stop email sharing from making outsiders full company members

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | high | missing-function | US-046 |

## Problem

Inviting by email from a doc/sheet Share panel calls `inviteToCompany` (default role member). An outside accountant shared on one doc becomes a member who can open everything set to "Company can view/comment/edit". There is no guest, external link or single-item access, and Copy link grants nothing.

## Evidence

- `web/src/lib/components/DocumentShare.svelte:220-221`
- `web/src/lib/model/members.ts:146-151`
- `web/src/lib/model/general-access.ts:11-16`

## Suggested direction

Add item-scoped guest access or a revocable external link; say clearly when an invite grants membership.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
