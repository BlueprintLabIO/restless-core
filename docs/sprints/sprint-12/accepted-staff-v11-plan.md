# S12 accepted Staff contribution probe v11 — frozen plan

**Company:** `sprint12_accepted_v11_test` (disposable `_test` company)
**Purpose:** repeat the bounded v10 factual chain after correcting only its deterministic gate argument parsing. This is not a real-company outcome and does not replace the Cosmon or visual-review gates.

## Why v11 exists

The v10 real Staff model wrote and linked its exact evidence note, but three literal-file gates passed a dash-prefixed Markdown bullet to `grep` without its `--` end-of-options delimiter. `grep` therefore returned exit 2 before matching the already-correct file. The product result was a failed Attempt and an unintended lead recovery conversation, so v10 is counterevidence—not a passing Staff contribution probe. A read-only check against the retained v10 file confirmed that `grep -Fxq -- <bullet> <path>` returns zero.

V11 changes no product code, model instruction, or intended outcome. Its three dash-prefixed literal gates insert the proved `--` delimiter and use fresh company and output paths.

## Gap under test

The existing v3 run proves a lead can choose and commission a stable Staff seam; v9 proves a Work-linked mid-work decision can be received by the same live Staff Attempt. This probe isolates the remaining factual chain:

```text
real Staff Attempt → linked, gate-checked evidence note
  → requires edge → real lead Attempt receives that exact artifact input
  → linked, gate-checked lead review states accept/revise/reject
```

No fixture creates an artifact, marks an Attempt complete, or writes a decision on the model's behalf.

## Fixed setup

- Use the existing `anthropic/claude-haiku-4-5` host-broker route in a company configuration with no credential bindings, approved parties, repository, browser use, or external effect.
- Create one standing team: `product-direction` is the accountable delivery lead and `evidence-writer` is the evidence-synthesis specialist.
- Create bounded Staff Work for `/company/outputs/sprint12-v11-staff-evidence.md`. Its deterministic gates require the exact title and three factual bullets. Each bullet gate is an argv command with `grep -Fxq -- <literal-bullet> <path>`. The real Staff model must write, link, and report the artifact itself.
- Only after that Work factually completes, create lead-owned Work with a `requires` edge to it. Its expected artifact is `/company/outputs/sprint12-v11-lead-review.md`. The real lead must inspect the automatically bound upstream artifact, state whether it accepts, revises, or rejects the Staff contribution, and link its own result. Its gate checks only that the review exists, has its title, and names one permitted disposition; it never supplies the disposition.

## Passing observations

1. The Staff Work has one real model Attempt, one available linked output, and passing exact-file gates; no host fixture or fake model supplies the file.
2. The lead Work claims only after the Staff Work is completed and records the Staff artifact as an immutable Attempt input.
3. The lead model links a review artifact and completes only after its own gate. The review explicitly names the Staff contribution as accepted, revised, or rejected from the bound evidence; only `accepted` fills the target gap.
4. No Exec model wake, owner intervention, owner/Exec mail, receipt, external effect, repository change, or extra Attempt is produced by the probe.

If either model fails to link its declared output, if the lead does not receive the Staff artifact as an Attempt input, if the review is not accepted, or if any forbidden activity occurs, record it as counterevidence and destroy only this named `_test` company after capture.
