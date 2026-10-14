# UXG-311 — Show which Work and customer an approval or external action belongs to

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | ux | US-197, US-200, US-208 |

## Problem

"First contact: party" approvals are projected with no Work id, authority decision-history rows are also written with no Work id, and external-action rows have no Work field. In the Inbox the owner judges a draft without the job it serves. In Activity, approvals and sends have no "Open work" link, unlike answered handoffs. Following a customer or job across Inbox, Work and Activity breaks at exactly the actions that reach the outside world.

## Evidence

- `crates/restless-engine/src/attention.rs:588-600` (approval_required item, work_id: None, title "First contact")
- `crates/restless-engine/src/attention.rs:2100-2103` (authority decision row work_id: None)
- `crates/restless-engine/src/company.rs:249-265` (ExternalActionRow has no work id)
- `web/src/routes/[companyId]/company/activity/+page.svelte:279-281` ("Open work" only when a decision carries workId)

## Suggested direction

Carry the originating Work/attempt id on prepared effects and approvals and render it as a link in the Inbox detail and Activity rows.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
