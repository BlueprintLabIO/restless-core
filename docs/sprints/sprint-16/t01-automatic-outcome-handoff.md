# S16-T1 — Deliver a qualified outcome review exactly once

**Layer:** OrgIntel + Owner cockpit.

**Observed friction served:** Dogfood 1's research Attempt produced at 03:02:09Z, but a prepared
owner review appeared only after an owner message woke the Exec.

## Outcome

For Work explicitly requiring owner review, a qualified ReviewTarget becomes one existing
owner-judgement outcome-review handoff without an owner prompt. The owner receives the native target,
evidence, exact judgment sought and observable continuation.

## Acceptance

- Use existing Work, Attempt, artifact reference, gate and handoff concepts; add no workflow or
  onboarding lifecycle.
- Exercise a produced Attempt with its declared ReviewTarget and required gates, with no owner tell
  or manual Exec wake, and observe one owner-review handoff.
- Repeat the completion signal and restart/recover the relevant process; observe no duplicate handoff
  for the same qualified outcome/revision.
- Exercise missing target and failed-gate paths; preserve a visible blocked state rather than inventing
  an outcome link or marking the Work complete.
- Preserve the owner boundary: no automatic acceptance, no inferred judgment and no market action.

## Deletion target

Manual final-message intervention and any research-specific review glue that duplicates the existing
handoff path.
