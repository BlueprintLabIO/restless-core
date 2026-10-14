# UXG-100 — Support transferring company ownership

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | missing-function | US-057 |

## Problem

The role select offers only Member/Admin, `mayManage` refuses any action on the owner, and the owner cannot leave. `GET /company/authority-owner` and `POST /company/authority-owner/transfer` exist but nothing in web/src calls them.

## Evidence

- `web/src/routes/[companyId]/company/members/+page.svelte:224-229,` :430-431
- `web/src/lib/model/members.ts:164-167`
- `crates/restless-owner/src/owner.rs` route "/companies/{company}/company/authority-owner/transfer"

## Suggested direction

Add an owner-only, explicitly confirmed "Make owner…" flow backed by the existing transfer route.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
