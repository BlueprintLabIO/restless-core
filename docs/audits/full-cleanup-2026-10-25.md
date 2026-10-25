# Full Core and Cloud cleanup — active work

Owner objective: fully action the seven remaining areas in the maintenance review,
fix the observed problems, verify the resulting behavior, and merge every fix into
the respective `main` branches. This checklist preserves that full scope across
work sessions. A passing narrow test does not complete a broader item.

Starting sources: Core `f28efc65`, Cloud `ec353a3` (25 October 2026).
The earlier seven repairs are recorded in `maintenance-2026-10-25.md`.

## Required outcomes

- [ ] Dependency security: audit every committed npm and Cargo lockfile, inspect
  the released container dependency surfaces, repair findings, verify compatible
  model/browser/framework behavior, and remove obsolete exceptions. A suppressed
  advisory or a newly written exception is not a repaired dependency.
- [ ] Terminal sign-in: throttle public starts by trustworthy client address,
  preserve waiting/approved requests under flooding, keep polling and approval
  available, and verify failure/retry and token authority boundaries.
- [ ] Engine and OrgIntel: inspect scheduling, wakes, Work/Attempt transitions,
  retries, concurrency and restart recovery; execute database-backed behavioral
  scenarios and repair observed defects. Verify useful continuation after failure.
- [ ] Authority and isolation: inspect and exercise grants, revocation, budgets,
  effect idempotency/unknown outcomes, credential custody and company boundaries.
  Repair problems and retain adversarial checks for the material invariants.
- [ ] Backup, restore and operations: run the isolated backup/wipe/restore
  scenario, verify data/permissions/credentials and recovery failures, and make
  the resource report finish or explicitly report unavailable observations.
- [ ] Owner and collaboration journeys: exercise desktop and mobile, multiplayer,
  documents/sheets permissions, concurrent edits and reconnects against isolated
  seeded companies; repair observed problems and rerun the affected journeys.
- [ ] Integration: rebase and verify coherent slices, merge them into both mains,
  check CI, remove this task's disposable resources, and audit completion against
  every outcome above. No unrelated live company or appliance state is disposable.

## Evidence and discoveries

- An OSV Scanner 2.7.0 binary was downloaded from Google's official release and
  verified against its release SHA-256 file. Its initial scan covered all 19
  committed npm/Cargo lockfiles (11 Core, eight Cloud).
- The initial Rust scan found RSA timing-side-channel and unmaintained
  rustls-pemfile dependencies, three ZIP extraction advisories, and older Tokio
  versions in two experiment lockfiles. Dependency paths and reachable behavior
  must be checked before choosing the repairs.
- Core's full device queue evicted an approved request. The regression failed
  with HTTP 200 before the fix. Core now refuses overflow with 429 and retains
  waiting/approved requests; expiry releases capacity. Five focused checks and
  the owner unit suite pass (145 passed, one pre-existing ignored scenario).
- Cloud now limits new device starts to one per minute per trusted client
  address, independent of the plane hint. Polls remain available; missing or
  failed limit observations return 503. Forty-two edge checks pass. Wrangler
  validates the exact committed binding, and its bundled Worker executed in
  workerd with start 200, repeat 429 and poll 200. This is local verification;
  no production Worker deployment has been performed by this task.

This file remains active until current-source evidence proves every required
outcome. Detailed machine-local scan/test output is kept outside Git; summaries
and exact merged revisions belong here as each outcome is verified.
