# UXG-172 — Warn when a new secret name overwrites an existing one, and hide raw references

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-093 |

## Problem

"Add secret" with an existing name silently overwrites it; "Stored elsewhere" shows raw `infisical:/…` strings; a standalone secret reads "No configured connection", which sounds like an error.

## Evidence

- `crates/restless-owner/src/owner_vault.rs:65-77`
- `web/src/lib/components/CompanyVault.svelte:44-50,` :255

## Suggested direction

Check names and confirm replacement; show plain-language sources.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
