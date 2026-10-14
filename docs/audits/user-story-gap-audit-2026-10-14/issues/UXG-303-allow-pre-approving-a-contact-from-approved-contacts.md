# UXG-303 — Allow pre-approving a contact from Approved contacts

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | low | missing-function | US-192 |

## Problem

The approvals grant route approves a party with no pending request, and `restless approve <party>` does exactly that. The web only grants from an Inbox request and lists approved contacts with Revoke. An owner who knows Exec will need to email their accountant must wait for the first ask instead of approving in advance.

## Evidence

- `crates/restless-owner/src/owner.rs:1579` (approvals/grant route)
- `crates/restless-engine/src/approval.rs:76-91` (grant needs only a party
- no pending request required)
- `crates/restless/src/main.rs:410-421` (CLI Approve <party>)
- `web/src/lib/components/CompanyLimits.svelte:385` (Approved contacts row)
- `web/src/lib/components/CompanyLimits.svelte:164` (only revoke is called there)

## Suggested direction

Add "Approve a contact…" to the Approved contacts row with an address field and the same confirmation weight as an Inbox approval.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
