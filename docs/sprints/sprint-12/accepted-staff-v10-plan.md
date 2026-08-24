# S12 accepted Staff contribution probe v10 — frozen plan

**Company:** `sprint12_accepted_v10_test` (disposable `_test` company)
**Purpose:** add one honest accepted-Staff evidence point to the Sprint 12 record. This is not a
real-company outcome and does not replace the Cosmon or visual-review gates.

## Gap under test

The existing v3 run proves a lead can choose and commission a stable Staff seam; v9 proves a
Work-linked mid-work decision can be received by the same live Staff Attempt. Neither run produced
a linked Staff artifact that a later accountable lead Attempt received and accepted. This probe
holds the natural-commission evidence separate and tests that missing factual chain directly:

```text
real Staff Attempt → linked, gate-checked evidence note
  → requires edge → real lead Attempt receives that exact artifact input
  → linked, gate-checked lead review states accept/revise/reject
```

No fixture creates an artifact, marks an Attempt complete, or writes a decision on the model's
behalf.

## Fixed setup

- Use the existing `anthropic/claude-haiku-4-5` host-broker route in a company configuration with no
  credential bindings, approved parties, repository, browser use, or external effect.
- Create one standing team: `product-direction` is the accountable delivery lead and
  `evidence-writer` is the evidence-synthesis specialist.
- Create a bounded Staff Work for the exact path
  `/company/outputs/sprint12-v10-staff-evidence.md`. Its deterministic gate requires the exact title
  and three factual bullets. The real Staff model must write, link, and report the artifact itself.
- Only after that Work is factually completed, create a lead-owned Work with a `requires` edge to it.
  Its expected artifact is `/company/outputs/sprint12-v10-lead-review.md`. The real lead must inspect
  the automatically bound upstream artifact, state whether it accepts, revises, or rejects the Staff
  contribution, and link its own result. Its gate checks only that the review exists and names a
  contribution disposition; it never writes that disposition.

## Passing observations

1. The Staff Work has one real model Attempt, one available linked output, and a passing exact-file
   gate; no host fixture or fake model supplies the file.
2. The lead Work claims only after the Staff Work is completed and records the Staff artifact as an
   immutable Attempt input.
3. The lead model links a review artifact and its terminal result completes only after its own gate.
   The review explicitly names the Staff contribution as accepted, revised, or rejected from the
   bound evidence; only `accepted` fills the target gap.
4. No Exec model wake, owner intervention, owner/Exec mail, receipt, external effect, repository
   change, or extra Attempt is produced by the probe.

If either model fails to link its declared output, if the lead does not receive the Staff artifact as
an Attempt input, if the review is not accepted, or if any forbidden activity occurs, record it as
counterevidence and destroy only this named `_test` company after capture.
