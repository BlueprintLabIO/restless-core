# UXG-164 — Let the owner add or edit the legal profile

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-089 |

## Problem

The Legal profile is read-only; empty state "No legal details yet" has no action, and no owner route writes `legal_identity`.

## Evidence

- `web/src/routes/[companyId]/company/charter/+page.svelte:326-341`
- `crates/restless-owner/src/owner.rs:1271-1500`

## Suggested direction

"Ask Exec to prepare it" handoff or an owner form backed by Authority.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
