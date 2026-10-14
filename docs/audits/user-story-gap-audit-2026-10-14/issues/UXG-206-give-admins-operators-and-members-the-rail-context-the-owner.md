# UXG-206 — Give admins, operators and members the rail context the owner gets

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | consistency | US-110 |

## Problem

Several rail aids are gated on membershipRole === 'owner' rather than on access: agent handoffs in the transcript, the "Under way" strip, app requests, set-aside question sync, and /goal and /loop. The # Work and document references are offered only with owner access. An operator, who may decide reviews, gets none of it. Distinct from UXG-114 (the @mention picker in People rooms) and UXG-054 (Work "Waiting on you").

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:235-239` (exchanges owner-only)
- `web/src/lib/components/ExecutiveRail.svelte:273` (commands owner-only)
- `web/src/lib/components/ExecutiveRail.svelte:486-492` (set-aside owner-only)
- `web/src/lib/components/ExecutiveRail.svelte:1338-1341` (WorkInFlight, AppRequestCards owner-only)
- `web/src/routes/[companyId]/+layout.svelte:1391-1399` (references only with ownerAccess)
- `web/src/lib/components/ExecutiveRail.svelte:199` (canDecide includes operator)

## Suggested direction

Gate each aid on the permission it actually needs and show members a read-only version where they lack it.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
