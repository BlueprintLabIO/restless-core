# UXG-138 — Replace the free-text "Vault secret name" with a picker or inline entry

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | ux | US-072 |

## Problem

For token apps the owner must type the exact name of a Vault secret created elsewhere, with no picker, link or existence check.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:507-514`
- `web/src/lib/components/CompanyVault.svelte:158-199`

## Suggested direction

Select existing secrets plus "Paste token" stored inline.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
