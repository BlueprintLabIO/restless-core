# UXG-319 — Bring brand drift decisions to the Inbox

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | ux | US-209 |

## Problem

When a corrected brand fact makes already-produced work stale, the drift finding and its migration decision appear only in a section of Company → Brand & voice. Attention has no drift kind and the Company tab gets no badge. product-capabilities.md presents owner-decided migration as a working capability, but the owner only meets it by opening that page.

## Evidence

- `web/src/routes/[companyId]/company/identity/+page.svelte:43-47` (openDrift computed on page)
- `web/src/routes/[companyId]/company/identity/+page.svelte:264-270` (only place drift is rendered)
- `web/src/routes/[companyId]/+layout.svelte:820-828` (no Company badge for drift)
- `docs/product-capabilities.md:45` (owner-approved migration claimed)

## Suggested direction

Project an undecided drift finding that affects shipped or in-review work as an Inbox decision linking to the affected artifact.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
