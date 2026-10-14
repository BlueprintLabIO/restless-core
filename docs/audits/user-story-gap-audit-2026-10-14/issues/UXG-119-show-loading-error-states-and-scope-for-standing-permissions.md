# UXG-119 — Show loading/error states and scope for standing permissions

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | ux | US-069 |

## Problem

The section renders only on success, so while loading or on failure it disappears and the owner may believe none exist. Rows show no scope, beneficiary or usage, and it sits at the bottom of Members.

## Evidence

- `web/src/lib/components/StandingPermissions.svelte:47-69`
- `web/src/routes/[companyId]/company/members/+page.svelte:489`

## Suggested direction

Add skeleton, failure+retry and usage meta; consider moving under Company authority.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
