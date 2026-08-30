# S25-T8 — Prove plane restart does not stop a cell

The invariant that decides whether the tiers are actually separate:

> Each tier must be independently restartable without losing data or work in the others.

**Observed friction:** today this fails in both directions, which is the clearest evidence the tiers
are fused — restarting `restlessd` takes down every company at once, and one company's stale config
prevented any of it from starting (fixed by S25-T1, from the other end).

**Layer:** Evaluation, across all three.

**Deletion target:** assumed independence; untested restart paths.

## Scope

- Wake a company, let it hold an Attempt, restart the account plane, observe the cell still running and
  its next Attempt proceeding.
- Kill a cell; observe no other company is affected and no owner surface degrades beyond that company.
- Restart the fleet tier; observe no running cell is interrupted.
- Capabilities must be short-lived and re-mintable, effects idempotent, and reconnection must reconcile
  unknown outcomes — this ticket is where those claims are tested rather than asserted.

## Acceptance

All three restarts, each with stated inputs and observed output, in one evidence record. A failure here
is not a test bug; it means a tier boundary is not real yet.
