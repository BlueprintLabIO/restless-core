# C-SH-01 — research cairn navigation

Status: matched pair and confirmatory replicate closed; B0 wins this cell

This is the second ordinary-team crossover cell: small coding/product work with two independently
acceptable seams and low shared-state coupling.

- `scenario.md` is the frozen owner outcome.
- `workload.md` records the two pre-run structural judgements.
- `evaluate.mjs` is the frozen external native evaluator. The harness stores it outside actor mounts,
  verifies its hash and runs it only after the candidate is closed.
- `evaluate-r2.mjs` is the corrected confirmatory evaluator. It observes animation across the rendered
  beacon subtree and uses an exact arrival expression; the original evaluator remains immutable.
- `runs.md` indexes the randomised B0/B1 order and immutable result evidence.

No arm may edit the scenario or evaluator. A harness repair that could alter an outcome invalidates
both arms and restarts the pair.
