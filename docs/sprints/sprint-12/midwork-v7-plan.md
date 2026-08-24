# S12 mid-work direct-message probe v7 — frozen plan

**Company:** `sprint12_midwork_v7_test` (disposable `_test` company)
**Purpose:** repeat the v6 material-update control after adding the existing-inbox capability cue to a
live Staff Attempt. This is a narrow runtime probe, not a real-company outcome or a new coordination
mechanism.

## Fixed setup and intervention

The actors, team, one Staff-owned Work, model route, no-effect configuration, and one Work-linked
injected fact are identical to [`midwork-v6-plan.md`](./midwork-v6-plan.md). The actual message body is:

> The landing seam’s frozen interface changed: `terrain_collider_id` is no longer available; use
> `landing_zone_id` instead. Decide the integration contract before I continue. Send only the smallest
> direct decision back to me; do not involve Exec or the owner.

The only changed product input is the specialist's normal shared-spine guidance: when its decision
depends on the accountable lead, it can inspect its own addressed inbox once at that decision point.
That is a targeted existing-CLI read, not polling, a timer, a second process, or a new state type.
The fixture Work explicitly asks for that one final decision-point read, so this run tests delivery
rather than a model's willingness to invent a command name.

## Passing observations

1. The original Staff Attempt is running when the fact is injected and remains the only Attempt.
2. The lead takes one direct coordination wake and sends a Work-linked factual decision only to the
   Staff owner; no owner or Exec mail/wake is created.
3. Before its terminal decision, the same live Staff Attempt reads the lead's message and no longer
   reports that it is waiting for that already-delivered fact.
4. No repository/Git change, artifact, receipt, external effect, duplicate attempt, or timer-driven
   wake is used to obtain the result.

Any missing live read, stale block, duplicate attempt, owner/Exec relay, or source change is recorded
as counterevidence. Destroy only this named `_test` company after capture.
