# Account start lane and the constrained-host resource gate (A0.3, A0.6)

3 November 2026 (Sydney). Replaces the start-coordination part of draft PR #548. The rest of #548
(process-count admission) waits on Cloud's task cap, which is not deployed.

## What changed

- **One start lane per account.** Starting or replacing a company computer takes one
  account-wide lane in `runtime::up_locked`. Simultaneous wakes and a release's roll-forward
  rebuilds now queue instead of charging their startup memory together. An already running
  computer never takes the lane.
- **Automatic wakes wait for memory.** A sleeping computer with owed demand stays asleep while
  the account pool is above its admission threshold. The demand is kept, and the next scheduler
  scan wakes it once there is room. Wakes the owner starts (opening a link, sending a file) are
  not held. They still queue in the lane.
- **Roll-forward waits for memory.** A rebuild that is due waits while the pool is short of
  memory. It reports `waiting_for_quiet` with that reason and uses the same six-hour bound as a
  busy company.
- **Release gate.** `scripts/verify-constrained-host.py` installs the exact pool Cloud's VM
  agent installs (`install_workload_budget`) for an 8 GB / 4 CPU host. It then runs several
  `_test` companies against a scripted model, and checks:
  - **two-chats:** two conversations at once in a 768 MiB Runtime both answer, with no child OOM.
  - **simultaneous:** every company asleep, then woken at once. All of them answer.
  - **idle:** a quiet minute makes zero model calls.
  - **restart:** the daemon restarts and the next turn works.

  Owner entry (health, principal, Inbox, Work) is probed every two seconds throughout. Any failed
  read, unanswered message, idle call or pool OOM kill fails the gate.
  `scripts/release/local/resource-gate.sh` runs it after the jank gate in `core-release.sh`. It
  needs passwordless sudo for one transient slice, plus Cloud's agent script.

## The "two chats stall a 768 MiB Runtime" report

This does not reproduce on main `1c93fabb`. With the production-sized pool and a 768 MiB
Runtime, the Exec and a teammate were messaged at the same moment:

- Admission ran the two turns one after the other (`company_slots` 1).
- The Exec answered at 16.8 s and the teammate at 30.9 s.
- The Runtime peaked at 653 MiB, with zero `high`, `max` or `oom_kill` events.

PR #514's actor admission fixed the stall. The earlier failure ran both harnesses at once. The
gate now guards this case.

## Evidence (local, scripted model, no provider tokens)

| run | binary | result |
|---|---|---|
| two-chats, 8 GB host | main `1c93fabb` | pass. Exec 16.8 s, teammate 30.9 s. Runtime OOM 0 |
| simultaneous + idle + restart, 8 GB, 3 companies | main | all three scenarios pass. 6 entry failures came from the fixture probing during its own deliberate daemon stop; the probe now pauses first |
| simultaneous, 4 GB host (2 GiB pool), 4 companies | main | pass. The pool reached `memory.high` **116 times** and peaked at 1,819 MiB |
| simultaneous, 4 GB host (2 GiB pool), 4 companies | this branch | pass. **0** `memory.high` events, peak 1,686 MiB. Answers took 16.8 to 39.2 s (main took 16.6 to 37.0 s) |
| full gate, 8 GB host, 3 companies + peer | this branch | pass. two-chats 13.4 s / 26.6 s. Simultaneous wakes answered in 16.4 to 22.6 s. Idle: 0 calls in 60 s. Restart answered in 18.2 s. 480 entry checks, 0 failed, max 2,419 ms. Pool OOM 0. Cleanup clean |

Each small-host comparison ran once, so timings vary between runs. The memory-gated wake did not
trigger in these runs. The lane alone removed the throttling. This is local evidence. It does not
cover a multi-owner Cloud, real provider quality or production.

## Still open

- Process-count (`pids`) admission from #548. It needs Cloud's task cap deployed first.
- Docker starts that outlive a daemon, which #548 recorded. They are unchanged here.
- Owner-started wakes during memory pressure are allowed on purpose, so that entry stays
  responsive. Their memory is bounded only by the lane and the pool's hard limit.
