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
- [ ] Resource efficiency (owner follow-up): inspect memory, process lifetime,
  service budgets and build concurrency; reduce measured host pressure while
  preserving active work and company data, and retain verified preventive bounds.
- [ ] Integration: rebase and verify coherent slices, merge them into both mains,
  check CI, remove this task's disposable resources, and audit completion against
  every outcome above. No unrelated live company or appliance state is disposable.

## Evidence and discoveries

### Owner-expanded sprint programme — 26 October

The owner explicitly expanded this to every Restless codebase and deployment
surface, including bugs, debt, inefficiency, leaks and operational jank. Work
continues in coherent sprints followed by a fresh audit. The inventory includes
source, tracked configuration, generated contracts, vendored first-party
libraries, images, deployment agents, CI/release scripts, local services and
resource ownership. An area is not closed merely because its unit tests pass.

| Sprint area | Audit and verification boundary | Current state |
| --- | --- | --- |
| Dependencies and distribution | Every lockfile, browser/model compatibility, OCI/SBOM scans, installers and published library graph | Repairs merged; cockpit framework/KaTeX and complete OCI qualification open |
| Identity and public entry | Edge/device throttling, verification/recovery, passkeys, session lifetime, invitations, billing return origins | Device fixes merged; Fleet framework PR under CI; remaining authority cases open |
| Scheduling and recovery | Engine/OrgIntel queues, wake ownership, retries, leases, concurrency, crash/restart and durable continuation | Database suites pass; source/fault audit still open |
| Authority and company isolation | Grants, revocation, budgets, effect unknown outcomes/idempotency, custody, shared issuer, tenant boundaries | Credential fixes merged; adversarial end-to-end audit open |
| Owner and collaboration | Desktop/phone, large seeded data, documents/sheets, permissions, multiplayer and reconnect | Fleet account journeys verified; full Core/company journeys open |
| Deployment and operations | Compose, VM/plane agents, release contracts, TLS/routing, secrets references, migration/rollback, backup failure recovery, monitoring | Backup round-trip and reaper fixes pass; local manifest drift reproduced; remaining deployment audit open |
| Resource lifetime and efficiency | RAM/swap/CPU, process and socket lifetime, pools, caches/images, bounded observations and cleanup failures | Host cleanup and preventive bounds merged; continued measurements and leak audit open |

Each sprint records its reproducer, changed behavior, verification limits and
merged revision. Re-audits compare exact current source/dependency/deployment
inventories, exercise relevant failure paths, inspect leftover resources and CI,
and carry unresolved findings forward. Production observations and disposable
local evidence are recorded separately.

Follow-up deployment finding: `deploy/local/.env.example` embeds an older
plane-deployment schema digest than the current contract and its named release
fixture. The actual current Fleet API refused it with “Fleet Cloud release
contract schema binding is invalid.” The temporary account-test manifest was
bound to the compiled schemas; the tracked local deployment input still needs
repair and a regression guard.

### Verified work

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
- That terminal edge repair merged in Cloud
  [`#142`](https://github.com/BlueprintLabIO/restless-cloud/pull/142),
  `13de28a002e78d964cb5b888b02536d0a82d9584`, after all three CI jobs passed.
- The public site now uses SvelteKit 3 and the matching static adapter. The full
  npm audit has zero findings. Artifact/type checks, 11 waitlist tests, four
  presence tests, production build and all 2,313 internal links across 54 pages
  pass. Desktop/phone browser inspection covers landing assets, category/article
  navigation and the Cloud form view without browser errors. Its existing
  verification now runs in CI. Cloud
  [`#143`](https://github.com/BlueprintLabIO/restless-cloud/pull/143) merged as
  `e3a92b2db08add1baf35d92a01113cd42004bbf4` with all three CI jobs passing.
- The ignored appliance backup scenario was explicitly executed against an
  isolated PostgreSQL 17 container: backup, wipe and restore passed in 45.53s,
  including persisted data, credentials and file permissions. Its disposable
  container was removed. Broader recovery fault scenarios remain to be audited.
- Reaper regressions reproduced two destructive responses to unavailable
  observations: failed container inventory deleted an isolated test company's
  configuration, and failed agent inventory deregistered its service. Both now
  preserve the affected resources. A stalled build-cache query also reproduced a
  blocked report. Docker health, inventory and expensive cache queries are now
  bounded; unavailable observations are reported as debt. All three isolated
  regressions pass with `python3 scripts/restless-reap.test.py`; shell syntax
  passes. A complete read-only host report reached the final summary and
  reported 23 items, including unavailable cache observations, existing test
  containers and build/storage debt. That observation preceded the owner's
  explicit authorization to purge aggressively, recorded below.
- Verified Rust repairs replace ZIP 2 with ZIP 9, remove rustls-pemfile,
  replace the direct test-only RSA generator and update both experiment Tokio
  locks. Core library checks passed with a real database (contracts 17, engine
  541, owner 145), as did all six runtime-bridge checks and both experiment
  checks. The existing CA validation was retained and exercised against valid
  certificates, malformed PEM, private keys and excessive certificates; the
  bridge now chooses its existing Ring provider explicitly instead of panicking
  when multiple providers are enabled. ZIP traversal and GitHub signing checks
  pass. The OrgIntel database run covers all 57 integration files plus library
  checks: 247 passed, one existing ignored test remains. Its old lead-only reply
  assertion contradicted the documented producer/lead/Exec rule; the replacement
  checks those three roles and rejects unrelated Staff. No conversation authority
  was broadened to satisfy the test.
  The scan still reports RSA through SQLx's optional MySQL lockfile entries in
  both repositories. Cargo's active all-feature/all-target dependency trees do
  not include RSA after the direct test dependency is removed, but the advisory
  remains recorded; no exception has been added to hide it.
- The browser-tool repair keeps OMP 18.8.6 and overrides Puppeteer to 25.16.0,
  which resolves chromium-bidi 157.0.8090-0 and Zod 4.6.7. Development tools
  audit clean. The company-image install reads the same flat override; a real
  installation probe built from that exact Dockerfile block has those versions.
  Its isolated Chrome 156 run passed CDP typing/clicking, a phone screenshot,
  cookie isolation, reconnect, BiDi mapping and OMP's actual browser launcher.
  The existing namespace protocol smoke passes. Fresh Bun installs from the image's
  frozen graph pass for Linux and for macOS arm64 dependency selection without
  changing the lock. The macOS installer now copies and freezes that release
  graph. This verifies dependency selection, not execution on a macOS host.
  The Zod 3 scan exception was removed. The probe extends an existing company
  image; full current-source release-image qualification remains a separate gate.
  This repair merged in Core `85ef06b6`.
- GitHub's 17 open Core Dependabot alerts were compared to the actual lockfiles
  and advisory ranges. All refer to already-replaced versions: the current
  graph has Vite 8.3.5, devalue 5.9.4, source-map-js 1.2.2, MCP SDK 1.32.2 and
  Hono 4.13.13. No alerts were dismissed. Cloud's Dependabot-alert API reports
  that alerts are disabled; npm/OSV and release-image scans remain necessary.

## Owner-authorized resource purge

After the owner explicitly requested an aggressive purge, exact Docker identities,
namespace labels and current process inventories were checked before removal.

- Removed 15 idle disposable test containers and their 15 exact, unreferenced
  test volumes. No blanket Docker volume prune ran.
- Stopped two abandoned development Documents sidecars that were crash-looping
  against a missing issuer. Their backing databases and company files remain.
- Removed three abandoned job-specific BuildKit builders and their caches after
  checking age and build history. The shared release builder and Dagger engine
  remain because releases are actively building.
- Removed the six reported cold compiler caches after checking resolved paths,
  process use and running-container bind mounts: 36,622,110,720 bytes. The four
  warm caches and every source checkout remain.
- Completed the reaper's Docker cache prune with a 20 GB target. Pruning unused
  images older than 24 hours additionally reported 3.607 GB reclaimed.
- Measured free space inside WSL rose by approximately 64 GiB, to 287 GiB. The
  complete post-purge report has no orphaned configs, exited/idle/long-running
  test containers, abandoned sidecar restarts, leaked services/processes or
  scratch worktrees. Warm compiler caches total 58 GiB, under the 60 GiB budget.
- Three observations remain in that report: current release-engine CPU use,
  an unavailable Docker disk-usage query and WSL virtual-disk slack. Physical
  Windows-drive reclamation requires a separate WSL compaction; no active
  release or production process was interrupted to compact the disk.

## RAM and efficiency follow-up

- Initial Windows free physical memory was approximately 8.5 GiB, with WSL
  resident at roughly 35 GiB. Linux had about 24 GiB available, much of it file
  cache. Memory pressure and anonymous usage were measured separately from
  cache; no `swapoff`, blanket process kill or live-WSL restart ran.
- Stopped 49 dormant development containers: two old vault/database stacks and
  their test sidecars, then a 39-container development profile whose host daemon
  was gone and whose company containers had no active agent. The stable owner's
  daemon was verified to use its separate `~/.restless` home. All stopped
  development containers, databases and volumes remain recoverable.
- Requested 2 GiB of kernel file-cache reclamation with `swappiness=0`, limited
  by the observed inactive file cache. The request completed; swap use did not
  increase and cgroup OOM counters did not change. Subsequent Windows free RAM
  was approximately 17 GiB, and Linux memory-pressure averages reached zero.
- Backed up the existing `.wslconfig` and staged a 32 GiB ceiling for the next
  normal WSL start, retaining its existing CPU, swap and gradual reclaim settings.
  The live VM was not restarted; its current ceiling therefore remains 40 GiB.
- The shared native builder now has a verified live 20 GiB RAM/total-swap limit
  and CPU shares of 256. Its cache was preserved and the existing DNS/TLS/token
  preflight passes. The wrapper applies those bounds on subsequent invocations.
  A regression reproduced two concurrent initializers deleting each other's
  shared `.next` file; unique temporary files now allow both callers to finish
  with exactly one builder creation.
- Infisical's Compose services now have RAM, CPU, process and swap bounds. Its
  first 512 MiB V8 heap failed in the isolated startup test and was discarded.
  The final 768 MiB heap under a 1.5 GiB backend limit passes real provisioning,
  authenticated secret writes, stop/restart, credential continuity and read-back.
  The test removed its containers, volumes and network. The live owner vault
  was not restarted to apply the new deployment settings.

## Still open

The cockpit's framework/KaTeX chain, Fleet Web's framework migration, the
remaining Rust/OCI security findings, the rest of the
engine/authority/recovery audits and full seeded owner/collaboration journeys
remain open. A repaired site or a passing library suite does not close them.

This file remains active until current-source evidence proves every required
outcome. Detailed machine-local scan/test output is kept outside Git; summaries
and exact merged revisions belong here as each outcome is verified.
