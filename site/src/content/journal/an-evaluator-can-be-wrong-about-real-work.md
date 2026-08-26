---
title: "An evaluator can be wrong about real work"
deck: "Four departments completed their exact units. A malformed review response made the company comparison invalid rather than successful."
thesis: "Product truth and evaluation truth need different terminal states, because a broken judge cannot erase observed work or approve it."
publishedAt: 2026-08-26
order: 4
readTime: "5 min"
status: "Accepted direction"
experiments: ["EXP-03", "EXP-05"]
evidence:
  - label: "Evaluation infrastructure result"
    locator: "experiment/coordination/experiments/EXP-03/t23-final-results.md"
    scope: "Corrected evidence and completion-path safeguards before final cells."
  - label: "Concurrent-company result"
    locator: "experiment/coordination/experiments/EXP-05/t04-company-concurrency.md"
    scope: "Ninety exact units and an invalid semantic review branch."
  - label: "Programme conclusion"
    locator: "experiment/coordination/experiments/EXP-05/t05-final-results.md"
    scope: "A controlled company run with no external effects."
---

An experiment needs a result about two things: what the company produced and whether the evaluation
was capable of judging it. They can diverge. Treating both as one success flag loses the next action.

## What happened

The EXP-05 company run placed four independent outcomes in one company window: sales, support,
monitoring and operations. One lead and one worker owned each outcome. The product path completed 90
of 90 exact units, and the Exec had returned to availability before a fourth request arrived.

Then the semantic reviewer omitted a required decision field. A bounded retry omitted it again. The
company evidence was still useful. The comparison evidence was not. The branch stopped as
evaluation-infrastructure-invalid.

Two tempting responses would have turned that into a neater story. We could have parsed surrounding
prose and inferred the missing field. Or we could have rerun the production branch until the reviewer
returned valid structure. Both would let the evaluator edit history after seeing the outcome.

## What changed

Restless now preserves a useful artifact, a rejected outcome and an invalid evaluator as different
terminal facts. A failure in the company should change the work. A failure in the judge should repair
the measurement boundary. Neither should silently reclassify the other.

The run also exposed a separate product fault. The fourth owner request took more than a minute to
dispatch even though Exec's organisational boundary was correct. Durable responsibility needs to be
accepted before lead orientation completes, so portfolio availability is not held behind internal
setup.

## Where the conclusion stops

The exact 90-unit outcome was a controlled test-world artifact. It does not prove a real company can
operate four departments or establish customer demand. It does establish a rule for evidence: a judge
may reject a result, but it cannot manufacture a success or make observed work disappear.
