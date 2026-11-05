# Sprint 111: safe updates

**Status: implementation and isolated verification complete; production rollout and operator staging proof in progress (5 Nov 2026).**

Implemented per-company recovery gates, bounded inventory/orphan/start/drain waits,
five-minute Work drain, ten-minute measured-memory recovery, durable half-budget
owner/operator notices, and a least-busy-first Runtime canary with a real Exec
turn. A failed Runtime canary restores its observed previous image and stops the
remaining rollout; company data volumes stay in place. Repeated bootstrap calls
are limited to two Authority transactions, leaving connections for entry and
readiness, with a thirty-second overall deadline and one-second dependency probes.

Validation: `scripts/verify-safe-update.py` ran against real Docker and isolated
PostgreSQL with two disposable companies and a local scripted provider. The broken
image rolled back, its neighbour retained the old image, both durable volume
markers survived, then a good canary made a real Exec inference before both
companies updated. Both authenticated test-company principal reads passed and
fixture cleanup left no resources. Recovery deadline tests prove the half-budget
alert fires once and even a stuck alert cannot extend the deadline. Production
entry is a separate check and is not established by these results.

The real retained-memory fixture (`scripts/verify-memory-recovery.py`) also
passed on 5 November: two companies were created after daemon startup, an idle
Runtime retained 1 GiB under a real 1.5 GiB/one-CPU workload cgroup, the durable
owner notice arrived at 308 seconds and only that Runtime restarted at 617
seconds. Both authenticated principal reads stayed available throughout, its
neighbour was never restarted, both volume markers survived, and cleanup left
no resources. Recovery now reloads the catalogue and accepts a measured idle
Runtime as a blocker. The control-plane health stage opens before independent
company cleanup, so the release agent does not mistake it for a global barrier.

The bar is the industry standard for a platform that updates twice a day: an update never takes
every company down at once, no step waits forever, and when something does stick, the owner sees
exactly what and has a door out.

## Why: the 4 Nov rollout

Core `ab5872b4` reached the owner plane at 20:19 AEDT. All six companies then showed "Updating" on
the account page and could not be opened for more than 50 minutes. The plane's `/health` stayed at
`roll_forward.stage = waiting_for_startup_recovery` with `detail: null`, and `busy` named one Work
running in the Restless company.

The likely cause, read from `crates/restlessd/src/main.rs` (startup recovery), not yet confirmed
from the plane's logs: after the Docker inventory, recovery loops on
`runtime::workload_admission(..)` until no running company reports `waiting_for_memory`, sleeping
5 s between checks. That loop has no deadline, sets no stage detail, and one busy company holds
every other company's entry closed. Nobody could see the plane's logs or restart it: the provider
token was refused and there is no break-glass access to owner planes.

## Gold standard against where Restless is

| Practice | Standard | Restless on 4 Nov |
|---|---|---|
| Every wait has a deadline | Drain, start-up and health waits are bounded; past the bound the system acts (restart, kill, roll back) | The memory-headroom loop is unbounded |
| Automatic rollback | A version not healthy within N minutes goes back to the last good one, with no human | None |
| Tenant isolation | One tenant's load cannot block another; each has its own memory limit and restarts on its own | One busy company held all six |
| Progressive rollout | One tenant (canary) first, checked, then the rest | All companies at once |
| Checkpoint, don't wait | Long jobs checkpoint; a restart resumes instead of waiting for work to end | Chat and Work are durable, but the update still waits on running Work |
| Honest status and alerts | A precise reason and age on screen; an alert when a stage passes its budget | "Can take a few minutes" for 50 minutes, no alert |
| Break-glass access | Operators can always read logs and restart, with a runbook | No access to owner planes |
| Updates never close entry | The durable owner plane stays readable while runtimes finish behind it | Entry closed for every company |

## Work, in priority order

### 1. Deadlines and automatic rollback on every update stage
- Give every roll-forward stage (drain, startup recovery, memory admission, orphan sweep, runtime
  start) a deadline. Ten minutes is the default for the memory wait.
- Past a deadline: force-restart the stuck company's runtime, letting its Work resume (see 3).
  If the plane itself cannot reach healthy, roll the plane back to the previous revision.
- Record each deadline hit in the release log and as a plane event.
- **Done when:** a test plane with a company that never frees memory finishes its rollout within
  the deadline, the other companies open as soon as they are ready, and the stuck one is restarted
  or reported.

### 2. Per-company isolation during an update
- Each company's runtime runs under its own memory limit (cgroup), so one cannot starve the plane.
- Recovery and admission run per company. A company that is ready opens; one that is waiting
  never blocks the others.
- **Done when:** the 4 Nov case (one busy company) leaves the other five usable throughout.

### 3. Resume instead of waiting
- An update may interrupt running Work after its drain deadline. The Work resumes from its last
  durable step on the new version (the interruption-resume path, bounded resumes, already exists).
- The drain budget is short and explicit, and is shown to the owner.
- **Done when:** a long-running Work interrupted by an update resumes on the new version, with
  its chat and outputs intact, and without a duplicate external effect.

### 4. Honest status and alerts
- `roll_forward.stage.detail` always names what it waits for, with numbers (for example "waiting
  for memory: Restless company, 7.4 of 8 GB in use, 6 min") and how long it has waited.
- The account page shows that sentence per company, and "Updating" never shows for longer than
  the deadline without saying why and offering a door (Restart this company, or Details).
- An alert (owner notification and an operator alert) fires when a stage passes half its deadline.
- **Done when:** every stuck state on screen names its cause and its door, and the alert fires in a
  test.

### 5. Break-glass access to owner planes
- An operator path to read a plane's daemon logs and restart its daemon or one company's runtime,
  with key-scoped, audited access (no shared root password).
- A short runbook in `restless-cloud/docs/runbooks/` for "a rollout is stuck".
- **Done when:** the runbook's steps work against a staging plane end to end.

### 6. Canary rollout
- A release reaches the least busy company first and is checked (health, entry, one Exec turn),
  then the rest. A failed canary stops the rollout and rolls the canary back.
- **Done when:** a deliberately broken release stops at the canary, and the other companies never
  leave the old version.

## Out of scope
- Multi-plane failover.
- Zero-downtime database migrations beyond what the current schema steps already do.

## Operational defaults
- Work drains for five minutes; shared-memory recovery acts after ten minutes.
- Existing Company → Computer owner recovery remains available. Emergency
  `platform-recover` is restricted to the authenticated local operator.
