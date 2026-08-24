# S14-T1 — Serialize charged model turns per company

**Layer:** Authority Plane model metering.

**Observed friction served:** S12’s Staff fuse gives every active metered session the same remaining
company ceiling. Two concurrent sessions can therefore each burn that remaining amount.

## Outcome

Exec and Staff acquire one shared per-company in-process lane before starting a charged provider turn.
A waiting turn does not open a model session; after the active turn releases its lane and records
usage, the next turn recalculates the remaining envelope. Subscription sessions are not serialized.

## Acceptance

- The lane is owned by the existing `SpendLedger` / meter boundary, not OrgIntel, the Runtime, or a new
  scheduler.
- Exec and Staff share exactly the same company key and lane.
- The permit covers only a metered provider session; provider-auth failure, cancellation and failover
  release it deterministically.
- A focused async test proves a second metered turn waits while the first holds the same company lane,
  proceeds on release, and does not block another company.
- A focused test proves subscription billing does not acquire the charged lane.
- The existing in-turn cumulative-cost fuse remains active and uses the actual remaining envelope after
  admission.
- Documentation states the residual one-active-turn provider-reporting overshoot; no durable
  reservation, queue, new Work state or policy language is introduced.

## Non-goals

- claiming exactly-zero budget overshoot;
- a durable reservation/lease table;
- fairness, priority, timeout, retry or staffing policy;
- serialising ordinary Runtime work or subscription usage.

## Deletion target

Independent concurrent metered-turn snapshots that multiply the remaining company budget.
