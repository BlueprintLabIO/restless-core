# Dogfood 4 after-action — Swift Arrival walking skeleton

**Status:** v0.4 technical evidence retained; EXP-11 experimental successor returned for revision
**Scenario:** [Dogfood 4](./dogfood-4.md), versions 0.4–0.5 experimental
**Company:** `swift_arrival_test`
**Runtime artifact:** `/company/projects/swift-arrival`, commit `3dc502a`

## What exists

A local Godot 4.7.2 two-player ENet delivery loop.  One host and one client connect, a client
uses the host-resolved crate interaction, takes the driver position, moves the bounded route,
releases the crate in the destination zone, and receives the host-owned completed-delivery state
on both peers.

Two fresh bounded probe runs passed, including an independent run.  The current project’s native
review target is `DISPLAY=:1 ./run-demo.sh`; captured fallback frames show connection, carried
cargo, route end, and delivery completion from host/client views.

## Evidence

Runtime evidence is committed with the artifact under `/company/projects/swift-arrival/evidence/`:

- `live-probe-{host,client}.log` — fresh pre-review PASS pair;
- `independent-{host,client}.log` — independent PASS pair;
- `review-shots/` — five rendered checkpoints;
- `verification.md` — the executable probe results and retained failures;
- `owner-attention.md` — two owner process rescues, a verification-policy correction, and the
  lost-worker recovery, stated as failures rather than success evidence.

The probe has a `--check-only` GDScript preflight and a 90-second watchdog.  This was added after
early parse failures and hung runs made the owner the timeout mechanism.

## Version 0.4 correction evidence

Founder trial found that the previous target was effectively a chase-camera demonstration and that
the remote desktop did not offer a reliable ownership flow for direct play. The correction replaced
the client presentation with a first-person view and visible delivery gloves, made the `WASD`/`E`
contract explicit, and added a local rendered X11 replay. The exact replay at runtime commit
`84ff174` passed `DISPLAY=:1 ./playtest.sh evidence/v04-os-input-pass`; its trace and separate host
and client logs prove OS-level input, host-resolved pickup, driver-seat entry while carrying cargo,
route completion, unload, and client-visible completion.

The first replay attempts failed visibly. They exposed both a title-selection issue in the harness and
an interaction defect: a carried crate always intercepted `E`, so the player could not enter the
driver seat. The evidence remains in the Runtime’s v0.4 directories; the successful repair is
`1f2cf6e`. This is the sort of product failure the prior headless probe could not establish.

Cockpit control now auto-claims only a free computer, renews only on observed desktop input, and
releases after 60 seconds idle. It refuses to displace an active actor or another owner tab.

An isolated Flash probe was intentionally attempted rather than assumed. Its fresh runtime first had
no OMP model-selection profile; after that profile was supplied, ACP initialization closed before
inference. Both attempts spent $0, so `zai/glm-5.3-flash` is unadmitted as a visual playtest Staff
worker for this run.

## What we learned about Restless

- A harness became an independently useful seam once repeated game failures needed bounded runs,
  distinct host/client logs, and durable negative evidence.  Separating it from gameplay was
  productive.
- The first production attempt repeatedly bypassed the intended probe and later ended without a
  trustworthy report.  The preserved Runtime tree, Git checkpoint, and recovery attempt retained
  the working game, but the recovery cost is a real Dogfood 4 finding.
- One provider stream ended without exact metering and triggered the spend fail-closed guard.  The
  owner audited it before clearing the pause; accounted spend then remained within the $25 ceiling.

## EXP-11 autonomous playability result

EXP-11 ran a larger isolated v0.5 experiment from exact v0.4 baseline `84ff1745`. One
non-producing Game Product lead and one end-to-end gameplay worker produced exact experimental
candidate `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`. It replaced clamp movement with physical player
and world collision, corrected camera-relative control, improved host validation and interaction
feedback, added recoverable cargo/seat paths, rejected the route-zero shortcut, and passed five final
deterministic gates.

The candidate is not accepted. A final fresh, source-blind exact GPT-5.6 Sol player proved the
shortcut rejection, then picked up, deliberately dropped and recovered the parcel, drove the loaded
truck to the rendered route-end state, exited, re-entered, moved again, exited again, and attempted to
unload in the destination structure. Delivery never completed. The independent referee and Exec both
recorded the same reproducible post-route-40 blocker.

This is a stronger result than the v0.4 walking skeleton and a negative product judgement at the next
frontier. It also shows why scripted completion cannot be called independent playability. See
[`EXP-11 RESULTS`](../../../experiment/coordination/experiments/EXP-11/RESULTS.md) and its
[`founder review`](../../../experiment/coordination/experiments/EXP-11/FOUNDER_REVIEW.md).

## Decision still owed

The v0.4 technical result remains historical. For the EXP-11 successor the prepared decision is
`revise`, and hands-on founder acceptance review is withheld because the independent usability gate
already failed. The result does **not** claim that the prototype feels good, that its hand/truck
presentation is sufficiently legible, or that it supports Internet or more-than-two-player
multiplayer.

Dogfood 4 remains the live Swift Arrival charter. Do not promote `41f4fa53`. If further work is
authorised, it should be one bounded route-end exit/unload repair and strict replication, not content
expansion or a new dogfood number.
