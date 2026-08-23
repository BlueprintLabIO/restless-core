# C-SL-01 — late guard

Status: running

This is the first counted ordinary-team crossover cell: small coding/product work with low useful
producer parallelism and high whole-artifact coupling.

- `scenario.md` is the frozen owner outcome.
- `workload.md` records the pre-run structural judgement.
- `evaluate.mjs` is the frozen external native evaluator. The harness copies it outside every actor's
  mounted context, verifies its hash, and runs it only after the candidate is closed.
- `runs.md` will index B0/B1 manifests, arm order, runtime identity and result evidence.

No arm may edit the scenario or evaluator. A harness repair that could alter an outcome invalidates
both arms and restarts the pair.
