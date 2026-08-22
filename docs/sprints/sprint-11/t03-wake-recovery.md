# S11-T3 · Close and recover every wake without polling

**Layer:** OrgIntel owns durable wake/Attempt facts; Runtime owns supervised ACP process lifecycle
and company reconciliation.

**Observed friction:** Daemon or container replacement could interrupt ACP after `wake` but before
`wake_end`, leaving the owner waiting and the singleton claim occupied. Exec also scheduled repeated
continuations while delegated Work already had a durable event that would wake it.

## Outcome

Every Exec wake and Staff Attempt reaches one terminal record or is detected and recovered after
restart. Delegated waiting is event-driven. Explicit stop/reconcile waits for supervised claims to
settle or returns a bounded conflict without replacing durable company storage.

## Acceptance

- Crash points before model start, during streaming, during a Staff tool and after termination output
  each recover exactly once.
- A waiting Exec creates no polling schedule; Staff completion wakes it.
- A true immediate continuation creates one bounded next wake.
- Stop → reconcile cannot overlap the old ACP process or duplicate a singleton Exec.
- Company volume, worktree, browser profile and already-recorded receipts survive replacement.
- Ambiguous consequential effects reconcile provider state before retry.

## Deletion

Makes stale in-memory claims, generic “Waiting for Exec” hangs and delegated-work polling schedules
deletable.
