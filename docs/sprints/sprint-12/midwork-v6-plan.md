# S12 mid-work direct-message probe — frozen plan

**Company:** `sprint12_midwork_v6_test` (disposable `_test` company)
**Purpose:** exercise Sprint 12 scenario D's delivery path after the v4 owner-mail leak and v5
coordination-boundary repair. This is a narrow communication probe, not a real-company outcome.

## Fixed setup

- Use the configured `anthropic/claude-haiku-4-5` subscription route through the existing host broker.
  The test-company config carries no credential or external-effect binding.
- Create one accountable `product-direction` lead and one `world-builder` Staff member in the same
  team. Create one bounded Work owned by `world-builder`, then wait until its first real ACP Attempt
  has a recorded process-start observation.
- Do not create a lead Work, a candidate artifact, a repository edit, an owner message, an Exec wake,
  an external effect, or a second Staff Attempt as part of the injected control.

## One frozen intervention

While that Staff Attempt is live, the test harness sends exactly one Work-linked message from
`world-builder` to `product-direction`:

> The landing seam's frozen interface changed: `terrain_collider_id` is no longer available; use
> `landing_zone_id` instead. Decide the integration contract before I continue. Send only the smallest
> direct decision back to me; do not involve Exec or the owner.

The harness injects this factual change so timing is controlled; the lead's interpretation and reply
remain a real model judgement. This is permitted only in the isolated `_test` company and is not
evidence of a model-originated message.

## Passing observations

1. The original Staff Work retains its one pre-existing Attempt while the message is sent.
2. The message is linked to that Work, wakes `product-direction` once, and the lead completes one
   coordination turn without an Exec terminal wake.
3. The lead sends a direct factual answer to `world-builder`; no message reaches the owner or Exec.
4. The coordination turn creates no candidate/review artifact, Git change, receipt, or new Work/Attempt.

Any early Staff termination, provider failure, missing lead wake, duplicate Attempt, owner/Exec mail,
unattributed file change, or unbounded retry is counterevidence/invalid infrastructure rather than a
pass. After recording the exact state, destroy only this named test company and its volume/schema/spend
spool.
