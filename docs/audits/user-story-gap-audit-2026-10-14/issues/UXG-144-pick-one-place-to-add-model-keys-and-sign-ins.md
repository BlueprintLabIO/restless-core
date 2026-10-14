# UXG-144 — Pick one place to add model keys and sign-ins

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | consistency | US-082 |

## Problem

API keys can be added in Account → Connections, Company → Intelligence → Connections (separate implementation) and Intelligence → Advanced → "Company-only API keys"; sign-ins also live in three places.

## Evidence

- `web/src/lib/components/AccountConnections.svelte:777-844`
- `web/src/lib/components/CompanyProvider.svelte:613-631,` :925-960

## Suggested direction

Keep account-level as the one place; link to it from Company.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
