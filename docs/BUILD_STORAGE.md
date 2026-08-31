# Build storage hygiene

**Status:** Working runbook  
**Observed:** 18 August 2026

This is repo-owned operating knowledge. The cleanup schedule and any macOS `launchd` configuration
are machine-local and do not belong in Git.

## Observed friction

The host data volume reached 429 GiB used of 460 GiB, with 296 MiB free. `~/Learning` occupied
201 GiB. The dominant physical allocations were regenerable Rust build trees:

- legacy `helm/target`: 158 GiB;
- `restless/target`: 24 GiB;
- two comparison-clone `target` trees: 12.8 GiB;
- scratch build trees: 1.2 GiB.

`cargo clean` reported 933,445 files and 239.7 GiB of logical artifacts in the legacy Helm target
alone. After removing the verified build outputs, inactive dependency trees, Docker build cache,
developer caches, and stale temporary clones, the volume had 219 GiB free and `~/Learning` occupied
711 MiB. Codex session history, Docker volumes, browser/application state, company state, and source
files were deliberately preserved.

This is the second observed disk-exhaustion incident. The sprint 06 run report records the first in
[`docs/sprints/sprint-06/run-report.md`](./sprints/sprint-06/run-report.md#health-gates-remained-fail-closed).

### Third incident — 29 August 2026, a different cause

The host reached 17 GiB free, below this document's own 30 GiB build floor, which failed
`restless-dev`'s preflight and blocked ordinary work. **Rust build cache was not the dominant
cause this time.** The host had accumulated, from ordinary `_test` company runs that were never
torn down:

- 79 orphaned `_test` company configs in `~/.restless/companies` (of 85 total);
- 51 `restless-vol-*_test` Docker volumes;
- 21 `_test` containers, 15 exited and 6 still running — one for two days;
- 18.98 GB of Docker build cache with zero active entries;
- a `cargo test --ignored` process leaked for 15h35m, holding two ports;
- an orphaned `restlessd.sock` from a daemon that died before binding it.

Purging the disposable set recovered **30 GiB** (17 → 47 GiB free) without touching the Rust
`target` tree, which stayed at 23 GiB and under this document's 30 GiB cleaning threshold.

The lesson is not a bigger cleaning schedule. Every one of these was created by a run that did not
survive to clean up after itself, and nothing reported the accumulation — the first symptom was an
unrelated build failing. The countermeasure is [`scripts/restless-reap`](../scripts/restless-reap):
`--check` reports debt and is read-only, `--purge` removes it, and `restless-dev doctor` prints the
report so it surfaces at the moment an operator is already looking at host health. It refuses to
touch any company without the `_test` suffix, removes volumes only by exact name, and reports
running containers rather than killing them.

## The second dimension: CPU and memory

The reaper above bounds what accumulates on *disk*. A later incident showed that was half the
problem. Host load reached 19 on 12 cores with swap exhausted, while `restless-reap --check`
reported clean, because every check it ran asked "does this still exist?" and none asked "what is
this costing right now?"

Three separate defects combined:

- **Containers had no resource bounds.** `docker run` passed no `--cpus`, `--memory` or
  `--pids-limit`, so one company could take the whole host. An abandoned Godot demo held ~6 of 12
  cores for 23 hours. Bounds now come from `DEFAULT_CPUS` / `DEFAULT_MEMORY` / `DEFAULT_PIDS_LIMIT`
  in `crates/restlessd/src/runtime.rs`, overridable per-run with `RESTLESS_COMPANY_CPUS`,
  `RESTLESS_COMPANY_MEMORY` and `RESTLESS_COMPANY_PIDS_LIMIT`. `--memory-swap` is pinned equal to
  `--memory` so a runaway is OOM-killed in its own cgroup rather than swapping the shared VM.
- **The staleness check never actually measured age.** It grepped `docker ps --format {{.RunningFor}}`
  for `days|weeks|months`, so a container at 24 h renders as `"24 hours ago"`, matched nothing, and
  reported clean — the advertised 24 h threshold was really 48 h, and `RESTLESS_REAP_STALE_HOURS`
  changed only the printed label. It now computes hours from `.State.StartedAt`.
- **Host-side process scanning cannot see into containers.** On macOS a container's processes live
  in the Linux VM and never appear in host `ps`, so the scan for leaked `cargo` processes was
  structurally blind to the dev servers and render loops that were actually burning the cores.

### Why killing the process did not work

The deepest cause was durable, not transient. `restless.conf` ends with
`[include] files=/company/services/supervisor/*.conf`, so an agent can register its own supervisor
program — and every observed one was written `autorestart=true`. A `kill` was answered with a
restart within seconds. Because the conf lives on the company's **named volume**, the leak also
survived container replacement and came back running on the next `up`. Nine such programs had
accumulated in one company, each pinning a throwaway worktree alive.

The reaper now deregisters rather than kills: `supervisorctl stop`, remove the conf, then
`reread && update`. It only does so for a container with **no attached agent**, since nothing in a
conf records which of them a live agent still needs. The include directory is the exact ownership
boundary — platform services (desktop, chromium, browser-broker) are declared in the main conf and
are never enumerated, so they cannot be stopped by this path.

## Diagnosis

This was build-cache accumulation, not evidence that Restless company data or the runtime leaked.
The Rust workspaces have no target retention or profile-size configuration. Repeated `cargo check`,
`cargo clippy`, and `cargo test` runs across the active repository, legacy repository, comparison
clones, and scratch experiments accumulated profile-, feature-, path-, and toolchain-specific
artifacts that Cargo does not bound by total size.

The daemon's 2 GiB free-space preflight is a last-ditch refusal to start more work. It detects an
already dangerous host; it is not a storage-retention policy.

## Working policy

1. Before a full Rust verification run, check the host with `df -h /System/Volumes/Data`. Treat less
   than 50 GiB free as a warning and do not start a full build below 30 GiB free.
2. Keep the active workspace's build cache for fast iteration. When its `target` directory exceeds
   30 GiB, clean it while no `cargo` or `rustc` process is running:

   ```sh
   cargo clean --manifest-path /absolute/path/to/Cargo.toml \
     --target-dir /absolute/path/to/target
   ```

3. Give throwaway clones and experiments a dedicated `CARGO_TARGET_DIR`. Remove that exact target
   with `cargo clean --target-dir ...` when the experiment ends. Do not leave comparison targets in
   persistent scratch directories.
4. Keep Docker build cache bounded rather than periodically deleting runtime data:

   ```sh
   docker builder prune --force --max-used-space 8GB
   ```

   Never automatically prune Docker volumes; an unused volume may still contain company data.
5. A weekly machine-local check should report filesystem free space, every workspace `target` size,
   and `docker system df`. It may clean only the exact regenerable locations above after confirming
   no relevant build is active.
6. Package-manager caches are secondary. When material, use their own cleanup commands (`go clean
   -cache`, `pnpm store prune`, `bun pm cache rm`) rather than broad deletion of user cache trees.
7. Never include Codex sessions, browser profiles, credentials, company state, Git worktrees,
   source, or untracked project files in automatic cleanup.

## Health integration

`restless doctor -c <company>` now treats less than 30 GiB of host headroom as degraded, and
`restless-dev` performs the same preflight before a build. A future machine-local weekly check may
warn near 50 GiB. This does not justify a bespoke storage manager or automatic deletion inside
`restlessd`; use ordinary Cargo, Docker, and operating-system scheduling.

The accepted trade-off is an occasional cold rebuild after threshold cleanup. The guarded risk is a
repeat disk fill. Automatic deletion of potentially durable state remains out of scope.
