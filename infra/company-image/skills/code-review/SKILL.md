---
name: code-review
description: Review a code change on two independent axes, standards and spec, without letting one review contaminate the other. Use before accepting a repository change whose correctness matters; the Restless form of popular two-axis code-review skills that fan out sub-agents.
---

# Code review

Two questions stay separate because each blinds the other:

1. **Spec**: does the change do what the Work outcome and its references require, for the real user?
2. **Standards**: is it correct, safe and maintainable in this codebase: bugs, error handling,
   security boundaries, tests for key invariants, local idiom.

## When one reviewer is enough

A small, low-risk change gets one review Work that answers spec first, then standards, in that order,
from the exact commit. Do not create ceremony for a one-line fix.

## When the change is material

The accountable lead commissions two review Work items owned by different actors. Both depend on and
revise the producer's Work so each receives the exact commit:

```sh
restless work add --owner <domain>-reviewer --role reviewer --title "Spec review: <change>" \
  --outcome "Judge only whether the exact commit meets the Work outcome and references when run natively." \
  --expected-artifact <report path> --requires <producer-work-id> --revises <producer-work-id>

restless work add --owner <domain>-auditor --role reviewer --title "Standards review: <change>" \
  --outcome "Judge only correctness, safety and maintainability of the exact diff; cite file and line." \
  --expected-artifact <report path> --requires <producer-work-id> --revises <producer-work-id>
```

Neither reviewer reads the other's report before submitting its own. The lead judges the combined
result; a disagreement is a decision for the lead, not a vote.

## How to review

- Check out the exact commit named in your bound inputs; never review a moving branch.
- Run the project's own gates and the change natively where it can be run.
- Report findings most severe first, each with the concrete failure scenario and its location.
- Request changes only for findings that matter to the outcome; style preferences that the codebase
  does not already enforce are not findings.
