# S12 mid-work direct-message probe v8 — frozen plan

**Company:** `sprint12_midwork_v8_test` (disposable `_test` company)
**Purpose:** test the deterministic late-feedback boundary exposed by v6/v7. This is a narrow
Runtime/OrgIntel probe, not a real-company outcome or a replacement communication protocol.

## Changed mechanism

The v7 control showed that a model-directed one-time inbox read can happen before a lead's direct
reply. The prompt cue is removed. Instead, when an Attempt reports a terminal result, OrgIntel checks
whether Work-linked feedback addressed to that Attempt's owner landed after its immutable input
snapshot. If so, that terminal report is recorded as `superseded`; the Work stays active and its one
ordinary scheduler successor receives the exact feedback as initial input. There is still one live
cognitive process per actor, no polling, no delay/timer decision, no second conversation process and
no new message or workflow type.

The test Work allows two Attempts solely for this sequential recovery. The first is expected to be
superseded after the direct update; the successor is the only permitted retry and must begin after the
first process ends.

## Fixed setup and intervention

The actors, team, model route, no-effect configuration, and one injected fact match v7. The first
Staff Attempt is instructed only to inspect current Work/team/People state and to make no file,
browser, network, message, owner, Exec, artifact or external-effect change. It must not poll, sleep,
or use a timer. The exact injected Work-linked message remains:

> The landing seam’s frozen interface changed: `terrain_collider_id` is no longer available; use
> `landing_zone_id` instead. Decide the integration contract before I continue. Send only the smallest
> direct decision back to me; do not involve Exec or the owner.

The harness sends that message only after confirming Attempt 1 is running. The lead's response and
both Staff terminal results remain real model behaviour.

## Passing observations

1. The direct fact wakes `product-direction` once; it sends one Work-linked response only to
   `world-builder`, with no owner or Exec message/wake.
2. If that response lands after Staff Attempt 1's frozen input cursor, Attempt 1 ends
   `superseded` rather than `produced`/`blocked`; its summary identifies the late feedback and no
   concurrent or duplicate Staff process starts.
3. Attempt 2 starts sequentially with that exact lead response attached as initial Work feedback,
   applies `landing_zone_id`, and reaches a truthful terminal state without stale-block narration.
4. No repository/Git change, artifact, receipt, browser/network action, external effect, timer-driven
   coordination wake, owner/Exec relay or further source change is used.

Any early first completion before the lead response, retry beyond Attempt 2, missing bound feedback,
owner/Exec interaction, concurrent process, or source change is counterevidence. Destroy only this
named `_test` company after capture.
