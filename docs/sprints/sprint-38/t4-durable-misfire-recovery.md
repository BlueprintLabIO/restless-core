# S38-T4 — Settle misfires and recovery exactly once

**Layer:** OrgIntel, Authority and execution supervisor

**Serves:** OS wake improves availability only when overdue work has explicit semantics and duplicate
execution remains impossible.

## Work

- Persist due consideration, claim, terminal outcome and next occurrence under one schedule identity.
- Implement `catch_up_once`, `coalesce_latest` and `skip_if_late` with a maximum-lateness bound.
- Reconcile daemon death before/after claim, duplicated wake, sleep/resume, cancellation, DST, clock
  movement and a large overdue backlog.
- Recheck current authority, budget and execution target before every recovered claim.
- Project local-machine and always-on timing requirements honestly to the owner.

## Acceptance

The frozen corpus proves every occurrence is either completed once or explicitly recorded skipped. No
case silently disappears, double-runs, replays an unbounded backlog or claims work while the Mac was
off. Restart can recover an owed claim without manufacturing completion.

## Makes deletable

Fixed-delay continuation schedules, in-memory-only due truth and arbitrary retry polling.
