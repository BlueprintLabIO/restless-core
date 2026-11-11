# EXP-17 controller protocol

**State:** Draft protocol; not executable authority

The authoritative contract is
[`exp-sprint-17-worker-architecture-benchmark.md`](../../../exp-sprints/exp-sprint-17-worker-architecture-benchmark.md).

Execution is further constrained by [`CODEX_PARITY_SPEC.md`](CODEX_PARITY_SPEC.md),
[`TASK_PORTFOLIO_SPEC.md`](TASK_PORTFOLIO_SPEC.md) and [`EVALUATION_SPEC.md`](EVALUATION_SPEC.md).
Where they differ, the experiment sprint and then this controller protocol win.

## Freeze manifest

Before counted work, write and hash:

- eight primary task instances, two transfer tasks and hidden fixtures;
- exact model/version/effort, runtime image, tools and network policy;
- common aggregate task ceiling and safety time envelope;
- native gates, blind rubrics, practical thresholds and tie policy;
- balanced arm order and opaque artifact labels;
- allowed neutral persistence for solo and worker actors; and
- event scripts for longitudinal instances.

Any change after the freeze is an amendment that invalidates and reruns both members of an affected
pair.

## Arm invariants

- `C`: one Codex actor owns planning, production and verification; no Restless organisational service.
- `R1`: Exec delegates to one non-producing lead; lead commissions exactly one identical Codex worker.
- `RP`: same as `R1`, with multiple identical workers only for predeclared locally closing units.
- `H`: not causal unless all parity fields match exactly.
- All arms share starting bytes, producing model/effort, native tools and total task ceiling.
- The controller supplies facts and kills the named process at frozen checkpoints but never gives
  semantic rescue.

## Execution order

1. Run the shared first-party Codex parity probe; a different producing harness in one arm blocks the
   experiment.
2. Run one balanced `C/R1` pair in each S-C, L-C, P-I and E-L family.
3. Apply the predeclared stop/continue gate before second instances.
4. Run `RP` only after its first P-I pair preserves quality.
5. Freeze process-blind scores.
6. Run L-C and E-L transfer pairs.
7. Reveal process metrics, classify failures and write the crossover guide.

Hidden chain-of-thought is never requested or used as evidence. Configured reasoning effort, messages,
tools, checkpoints, receipts, usage and outcomes are the observable collaboration record.

## Evidence isolation

Producers never see sibling-arm work, hidden fixtures, evaluator notes or prior scores. Evaluators see
only the frozen brief, artifact and allowed evidence until their scores are locked. Opaque artifact
labels are mapped to arms in a separate sealed manifest.

## Valid pair

A pair is valid only when start bytes, model/effort, tool/runtime envelope, task ceiling, event delivery
and evaluator contract match. A Sprint 26/runtime failure invalidates both sides for one repaired replay.
Product or coordination failure remains counted.

## Terminal decision

Write one decision per work-shape cell: `C`, `R1`, `RP` or `unknown`, with the paired quality, serious
blockers, cost, latency, recovery and transfer evidence. A pooled universal winner is forbidden.
