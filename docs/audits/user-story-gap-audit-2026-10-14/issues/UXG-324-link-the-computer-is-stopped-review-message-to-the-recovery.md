# UXG-324 — Link the "computer is stopped" review message to the recovery action

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | low | ux | US-204 |

## Problem

When an outcome review cannot open because the company computer is stopped, the item says "Start it from Company → Health" as plain prose. Health has recovery buttons, but the review item offers no link or action to reach them. The owner has to find Health in the Company sidebar, then come back to the Inbox to review.

## Evidence

- `crates/restless-engine/src/attention.rs:217-224` (unavailable detail is text only)
- `web/src/routes/[companyId]/company/doctor/+page.svelte:193-198` (recovery actions live on Health)

## Suggested direction

Attach an inspect action pointing to Health, or a direct start action, to review items whose target is unavailable.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
