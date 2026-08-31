# S22-T5 - Prepare owner review and after-action

**Layer:** Owner surface + Evaluation.

Present one healthy live candidate with a concise current owner brief. Preserve exact Core/Cloud commits,
review lineage, tool evidence, spend and remaining uncertainty. Record whether the Restless improvement
materially changed the resulting website and which machinery should be retained or removed.

**Observed friction:** the prior handoff exposed an invalid URL, dead service and implementation notes
instead of a dependable outcome.

**Deletion target:** manual handoff repair and unbounded evidence narration.

## Current evidence — 28 August 2026

- Owner handoff `08a0f84e-e684-4269-a1d7-b855f3972ab7` carries a current outcome-review brief and the
  exact current-attempt web artifact `http://127.0.0.1:4323/`.
- The old projection returned no ReviewTarget because evidence URL de-duplication occurred before
  attempt selection. The earlier attempt and current attempt shared the same URL, so iteration order
  could let the stale attempt consume it.
- `attention::select_review_artifact` now selects the exact available attempt independently and then
  evidence is de-duplicated for presentation. A repeated-URL regression test and strict loopback URL
  test pass; daemon and CLI build cleanly.
- The shared daemon was not restarted while unrelated company actors were active. Restless's existing
  isolated owner-surface proof server instead projected the same authoritative company and Runtime on
  ports 7888/7894 using the rebuilt code.
- That projection returned the exact current target as `available` and `runtime-web`. Issuing a review
  ticket succeeded, and an independent HTTP fetch through the opaque ticket host returned 200 with the
  rebuilt homepage body.
- The owner now has the one bounded accept/request-changes decision. No acceptance has been recorded on
  their behalf.
