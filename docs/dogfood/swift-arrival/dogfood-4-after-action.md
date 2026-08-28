# Dogfood 4 after-action — Swift Arrival walking skeleton

**Status:** Technical evidence complete; founder review pending
**Scenario:** [Dogfood 4](./dogfood-4.md), version 0.2
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

## What we learned about Restless

- A harness became an independently useful seam once repeated game failures needed bounded runs,
  distinct host/client logs, and durable negative evidence.  Separating it from gameplay was
  productive.
- The first production attempt repeatedly bypassed the intended probe and later ended without a
  trustworthy report.  The preserved Runtime tree, Git checkpoint, and recovery attempt retained
  the working game, but the recovery cost is a real Dogfood 4 finding.
- One provider stream ended without exact metering and triggered the spend fail-closed guard.  The
  owner audited it before clearing the pause; accounted spend then remained within the $25 ceiling.

## Decision still owed

The founder must record `accept`, `revise`, or `reject` after using the native review target.  The
technical result does **not** claim that the prototype feels good, that its hand/truck presentation
is sufficiently legible, or that it supports Internet or more-than-two-player multiplayer.

Dogfood 4 remains the live Swift Arrival charter.  A favourable review should extend it with a
new scenario version (for example, a narrow latency/physics run), not allocate Dogfood 5.
