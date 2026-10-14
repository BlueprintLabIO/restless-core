# UXG-051 — Add owner and status filters, and decide on the Work graph view

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | spec-drift | US-037 |

## Problem

Spec §12.3 requires filtering by goal, owner, status and priority and a Work graph as first view. The UI offers goal plus three fixed views; edges appear only as link lists on one detail page.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:99-107,` :195-207, :565-575
- `docs/specs/owner-cockpit.md:636-650,` :1309-1320

## Suggested direction

Add owner/status filters in the URL; build the graph or amend the spec.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
