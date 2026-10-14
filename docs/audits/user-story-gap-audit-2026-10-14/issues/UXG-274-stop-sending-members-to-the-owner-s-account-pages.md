# UXG-274 — Stop sending members to the owner’s Account pages

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | consistency | US-172 |

## Problem

Every member’s profile menu links to /account, whose frame unconditionally loads the owner-only company list and the owner’s account connections. The company shell itself is careful to gate the same company list on owner access, so the Account page is the one place a member walks into refused reads and owner controls ("Give access", "Companies could not be loaded"). It reads as broken rather than as not theirs.

## Evidence

- `web/src/lib/components/ProfileMenu.svelte:50` (Account link for everyone)
- `web/src/lib/components/AccountFrame.svelte:81` (companiesQuery() not gated)
- `web/src/routes/[companyId]/+layout.svelte:169` (shell gates the same query on ownerAccess)
- `web/src/lib/components/AccountConnections.svelte:618` (companies could not be loaded message)

## Suggested direction

Hide or replace the Account link for non-owners with only what applies to them (appearance, notifications, sign out).

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
