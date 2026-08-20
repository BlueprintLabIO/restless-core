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
