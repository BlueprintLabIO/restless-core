# UXG-171 — Let the owner delete a Vault secret and replace any secret from its row

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | high | missing-function | US-093 |

## Problem

The owner API has only `GET /vault` and `POST /vault/secret`; secret rows have no actions; replace is hidden behind Add secret and only lists secrets wired to a connection. Old tokens for removed apps stay forever.

## Evidence

- `crates/restless-owner/src/owner.rs:1391-1395`
- `web/src/lib/components/CompanyVault.svelte:36-42,` :204-213, :231-237

## Suggested direction

Per-row Replace and Delete (confirmed, warning about users) backed by a delete route.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
