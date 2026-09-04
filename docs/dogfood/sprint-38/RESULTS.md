# Sprint 38 results — dependable local appliance

**Observed:** 3–4 September 2026

**Counted host:** macOS 26.6.2, arm64

**Decision:** implementation passes; terminal release decision remains pending on seven elapsed days

## What is now true

Restless installs as one per-user `launchd` appliance with a stable owner origin, a versioned release,
an independent wake-only adapter and an exact singleton lock. Stable, development and test planes use
different state, database, socket, port, log, launch-cache, container and volume identities.

Startup no longer makes the account plane hostage to Docker Desktop. The durable owner and control
surfaces open first in an honest `recovering` state. New work, manual launches and the scheduler remain
behind a separate recovery admission barrier. One bounded Docker inventory retries in the background;
effect and cognitive orphan sweeps run from that shared inventory, and admission opens only after they
finish. Owner health reads fail fast while recovery owns Docker, so an eager browser cannot starve it.

The owner portfolio now reads all configured Runtime states in one exact batch and paints company rows
before deeper Cockpit and Attention projections settle. A failed Runtime observation produces an
`unavailable` status, not a blank portfolio.

## Counted evidence

| Lane | Result |
| --- | --- |
| Install / upgrade | Latest live activations reached the control socket in 1.74–2.65 s, below the 30 s bound. |
| Congested recovery | Before probe isolation, safe recovery took 84.5 s while the owner stayed responsive and work stayed closed. With isolation it completed in 19.4 s; later starts completed in 3.7–12 s. |
| Singleton crash | A forced daemon death produced one replacement in 2.460 s with no duplicate singleton. |
| Native wake | One wake returned in 0.767 s; duplicate delivery settled in 1.084 s without duplicate work. |
| Stable/dev/test isolation | Concurrent planes and adversarial cross-profile stop, attach, migration and cleanup probes could not name each other's resources. |
| Rollback | Backward activation completed in 0.88 s and forward activation in 3.20 s. `current` and `previous` swapped bidirectionally. |
| Uninstall | Completed in 0.55 s. Both LaunchAgents, binary/install root, socket, machine state and launch cache were absent. |
| Reinstall | Activated in 2.38 s and reached fully recovered `ready` after 12 s. |
| Durable truth | All six company configuration SHA-256 digests were identical before rollback, after rollback, after uninstall and after reinstall. Authority remained at 902 records. |
| Owner desktop | The portfolio painted usable company rows in 1.649 s; deeper projections filled asynchronously. |
| Owner mobile | At 390×844 the page had a 390 px client and scroll width, with no page-level horizontal overflow. |
| Company Computer | Resources & access exposed one enabled **Open** action and routed it to the private Company Computer with an explicit **Enter computer** control. |

## Launch and publication proof

The exact Swift Arrival macOS artifact passed the native broker lane before cleanup: its digest and
platform were verified, the client joined the matching server build, repeated Open reused one process,
no reusable invitation or model/provider credential appeared in argv or process environment, and the
only client material was an expiring opaque local handle. Five-minute expiry terminated the client and
made exchange return Gone. Wrong digest, platform, audience, revocation and expiry all failed before
game access.

The exact OCI Godot UDP fixture passed local publication with an OS-assigned host UDP port, bounded
resources, read-only filesystem, dropped capabilities, process supervision and terminal cleanup. The
embed-denial policy passes. A source-blind exact HTTPS embedded-web Open is not yet a counted live
product proof, so the release claim remains narrower than “all web artifacts open inside Restless.”

Founder-controlled Swift gameplay was not counted: the first visual run was blocked by the locked Mac,
and the disposable proof database later no longer matched the current migration set. The launch and
security broker proof is valid; the “complete one delivery” usability claim remains for the elapsed
dogfood.

## Bugs found and removed

1. Unbounded Docker children could wedge owner HTTP for minutes. All owner-facing Docker probes are
   kill-on-drop and bounded.
2. `launchd` `ProcessType=Background` froze the service under machine pressure. The plane is now
   `Interactive`; the low-priority wake helper remains `Background`.
3. Startup performed one Docker inspection per retained company and scanned every historical Attempt.
   It now performs one exact inventory and queries only running Attempts.
4. Recovery blocked the control socket and could make both candidate and last-known-good releases miss
   readiness. Recovery is now asynchronous, observable and independently gates work.
5. Eager Cockpit probes competed with startup recovery. Owner Runtime probes now defer while recovery
   owns Docker.
6. The portfolio withheld all companies until every deep projection completed. Catalogue truth and
   enrichment now settle independently.
7. Rollback pointed `current` and `previous` at the same release. Activation now swaps both pointers
   atomically and restores both on failure.
8. Published fixtures could leak waiters, use an unusable Docker UDP mapping, or retain dead receipts.
   Provider processes are supervised/reaped, UDP allocation is explicit and retryable, and receipts
   reconcile to live or recoverable truth.

## Known conditions, not hidden passes

- Several retained pre-sprint company cells are quarantined because migration 20 was historically
  applied with different bytes. Sprint 38 preserved them; it did not rewrite migration history.
- Some stable companies lack a currently usable configured model credential and are shown as
  `cannot start` with the exact repair reason.
- Linux service definitions have contract coverage, but this sprint counts live macOS only.
- Seven ordinary days, a real reboot, sleep-overdue wake and founder-completed Swift delivery cannot be
  compressed into the implementation run. They are tracked in `DOGFOOD.md`.

## Cleanup proof

The isolated Swift test daemon, publication fixture, test company container and test volume were
stopped and removed. The four Sprint 38 databases and generated test database role were dropped.
Transient screenshots, samples, native launch material, temporary downloads and build roots are not
committed and are removed after this result is reduced to text. The stable appliance remains installed
and running.

## Verification

- `cargo test --workspace` passed; the eight ignored tests require explicitly provisioned live
  credentials, a public callback or a dedicated live Runtime.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Cockpit `npm run check` and the production build passed with no Svelte diagnostics.
- The final installed release for day zero is `dc2d9f2b288e96a97ef4`; both the CLI and owner API report
  the stable appliance ready after recovery.
