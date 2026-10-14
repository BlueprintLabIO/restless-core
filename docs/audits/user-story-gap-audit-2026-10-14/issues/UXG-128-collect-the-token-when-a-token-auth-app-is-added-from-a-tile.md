# UXG-128 — Collect the token when a token-auth app is added from a tile or the ask box

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | high | missing-function | US-072 |

## Problem

`addCatalogue` sends no auth, so GitHub is created with no token. Afterwards the app page shows only "Check again" because the Vault-secret input renders only while the app is still available. The tile tooltip promises a secret choice that never comes; only Remove undoes it.

## Evidence

- `web/src/routes/[companyId]/apps/+page.svelte:99-115`
- `web/src/routes/[companyId]/apps/AppTile.svelte:231-235`
- `web/src/routes/[companyId]/apps/[app]/+page.svelte:502-514`
- `web/src/lib/model/apps.ts:122`

## Suggested direction

Send token-auth entries to the app page's Add step, as AppRequestCards.svelte:46-48 already does.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
