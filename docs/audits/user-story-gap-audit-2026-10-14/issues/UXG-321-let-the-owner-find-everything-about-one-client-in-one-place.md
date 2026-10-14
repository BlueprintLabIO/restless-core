# UXG-321 — Let the owner find everything about one client in one place

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | missing-function | US-208 |

## Problem

There is no client or correspondent anywhere in the product. People's "contacts" are Exec and team leads, and ⌘K searches Work titles, documents, sheets, comments and room messages, but not Activity parties, decisions or inbound mail. Preparing for a client call means checking four surfaces by hand and still missing the emails and payments that involved them. Distinct from UXG-175 (filtering Activity itself).

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:89-91` (contacts are Exec and team leads)
- `web/src/routes/[companyId]/+layout.svelte:1221-1227` (search sources exclude Activity/decisions)
- `crates/restless-engine/src/company.rs:249-265` (external actions carry a party that nothing indexes)

## Suggested direction

Make an external party searchable in ⌘K, and give each party a "Everything about X" view that gathers Work, documents, messages and Activity rows without adding a CRM entity.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
