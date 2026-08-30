# S25-T7 — Cockpit readable with every cell asleep

The owner must be able to open the cockpit, see every company and read recent outcomes, attention and
spend without waking anything.

**Observed friction:** the cockpit's reads assume a live cell. Without last-known projections held by
the account plane, an owner pays model spend merely to glance at their own business, and the cost of a
glance scales with how many companies they run.

**Layer:** Owner Cockpit + Authority Plane (projection custody).

**Deletion target:** wake-on-view and live-only cockpit reads.

## Scope

- The account plane retains each company's last known projection: outcomes, attention items, spend,
  runtime status, and the `unstartable_reason` from S25-T1.
- Cockpit views render entirely from projections; waking is one visible affordance that names the
  consequence.
- Staleness is shown honestly rather than hidden — a projection is labelled with when it was true.

## Acceptance

Stop every cell. The cockpit renders every company with real content and starts no container and no
model request. Verified headlessly by asserting zero spend rows and zero container starts across a
full cockpit load.

## Closure evidence

Satisfied structurally rather than by adding a projection cache: **a cell's database is a host
service, not part of its container**, so a stopped cell is still readable.

- The full cockpit projection for a company whose container has never existed returned
  `source_health: {authority: available, orgintel: available, runtime: absent}`.
- **0 containers started and 0 spend rows written** by that load.
- The portfolio row additionally renders `cannot start` with the exact reason as a hover explanation
  for a company the plane could not admit (S25-T1), so an unstartable cell is legible without opening
  it.

No projection-cache machinery was built, because none was needed — the acceptance criterion was met
by the storage boundary the sprint already introduced.
