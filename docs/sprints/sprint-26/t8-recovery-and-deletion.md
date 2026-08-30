# S26-T8 — Prove recovery and delete the escape hatches

Close the sprint with the old failure cluster, not with isolated unit claims.

**Observed friction:** the EXP-15 controller used manual branch resets, ownership repair, process
inspection, port cleanup, repeated gates and artifact relinking to keep useful work moving. Any one fix
can pass while the full system still churns.

**Layer:** Evaluation across OrgIntel, Runtime and Authority.

**Deletion target:** known-instance repair scripts, operator process/filesystem archaeology and shadow
paths made redundant by T1–T7.

## Scope

- Build one deterministic fixture containing wrong source, mixed ownership, generated cache, two
  concurrent native gates, mid-Attempt feedback and scheduler death.
- Run clean, concurrent and crash/restart variants at least once each after all earlier tickets land.
- Measure model turns, lead wakes, gate executions, leases, manual interventions and retained junk.
- Audit production callers and delete superseded paths; do not merely leave them unused in the test.
- Publish a compact evidence record and residual-risk list.

## Acceptance

- All Sprint 26 acceptance criteria pass in the integrated scenario.
- Zero manual repair, cross-Attempt connection, leaked child, false gate pass, half-promotion or raw
  capture in Git occurs.
- The deletion record names removed paths and any deliberately retained compatibility boundary.
- EXP-16 may start only after this ticket records a passing substrate baseline.

