# Rollback refuses a release that cannot open company data a newer one upgraded

Sweep finding #38. An older restlessd refuses any OrgIntel schema carrying a
migration it lacks (sqlx VersionMissing), boot only quarantines the cell, and
wait_ready checks liveness, so `restless appliance rollback` (and the
automatic rollback after a slow upgrade) reported success while every company
failed to open.

- OrgIntel::unknown_applied_migrations reads a cell's applied migrations
  read-only and names those this build does not carry.
- `restlessd appliance-preflight` refuses activation when any configured
  cell has one, telling the owner to keep a release that carries it or
  restore a backup. A cell it cannot read is left to boot, as before.
- `restless appliance rollback` runs the previous release's own preflight
  before draining or touching any pointer, and refuses without change.
- An upgrade whose candidate misses readiness restores the previous release
  only if that release's preflight passes; otherwise the candidate stays
  active and the recovery state says why.
- docs/self-hosting.md states when rollback refuses.

Accepted: a target release built before this change has no such check in
its preflight, so the first rollback to one is still unguarded.

Checks run:
- restless-cargo check -p restlessd -p restless: finished, no warnings.
- restless-cargo test -p restless appliance: 10 passed.
- restless-cargo test -p restless-orgintel --test smoke: 9 passed; the new
  newer-schema scenario compiled but skipped, as RESTLESS_TEST_DATABASE_URL
  is unset here (no PostgreSQL was started).
- Built restless/restlessd, laid out current/previous releases under a
  throwaway HOME with an unreachable OrgIntel URL, ran
  `restless appliance rollback`: exit 1, "cannot roll back; the current
  release stays active ... release failed its read-only preflight, so it was
  not activated", and the current/previous pointers and state files were
  unchanged. Throwaway HOME removed.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/smoke.rs`

```
use chrono::Utc;
use restless_orgintel::{ NewArtifactRef, NewGateRun, NewOwnerHandoff, NewWork, NewWorkGate, OrgIntel, OwnerBrief, OwnerBriefKind, OwnerHandoffCategory, OwnerHandoffState, OwnerReviewDecision, WorkAttemptState, WorkEdgeKind, WorkStatus, WorkspaceSpec, };
```
