# EXP-16 controller protocol

**State:** Staged controller protocol; counted execution remains gated

The authoritative contract is
[`exp-sprint-16-embodied-npc-playtesting.md`](../../../exp-sprints/exp-sprint-16-embodied-npc-playtesting.md).

Execution is further constrained by [`NPC_ARCHITECTURE_SPEC.md`](NPC_ARCHITECTURE_SPEC.md),
[`SCENARIO_SPEC.md`](SCENARIO_SPEC.md) and [`EVIDENCE_SPEC.md`](EVIDENCE_SPEC.md). Where they differ,
the experiment sprint and then this controller protocol win.

## Activation

Freeze before any counted call:

- Sprint 26 passing evidence identity;
- exact EXP-15 source commit/tree and immutable review target;
- canonical preset, baseline seeds and held-out seed commitment;
- model route, effort, tool envelope and aggregate budget;
- baseline commands and event receipts; and
- independent evaluator brief and rubric.

## Invariants

1. Exec delegates once to one non-producing Game Product lead and returns.
2. At least one Staff worker owns every product change; the lead does not edit or play.
3. NPCs act through normal action, physics and authority paths. Direct outcome mutation and evaluator
   hidden-state reads invalidate the run.
4. Fixed seeds reproduce decisions; held-out seeds are not exposed to the producer before freeze.
5. Headless mechanics run frequently. Vision is sparse and event-triggered, never frame-by-frame motor
   control.
6. Ordinary feedback enters the active Attempt at a safe checkpoint. Only an explicit urgent interrupt
   cancels it.
7. One product defect may produce one bounded repair Work. Infrastructure retries do not count as loops.
8. Exact candidate, gate and artifact lineage is mechanically enforced.
9. NPC pass does not establish fun, human legibility or founder acceptance.
10. Raw captures and transient process state are cleaned after compact evidence is retained.

## Stage gates

- **S0:** baseline valid and frozen.
- **S1:** shared body/action contract and anti-cheat suite pass.
- **S2:** delivery evaluator completes frozen and held-out routes.
- **S3:** recovery evaluator handles bounded adverse states.
- **S4:** robber behaviour creates fair success and adverse paths.
- **S5:** vampire behaviour creates cooperative and adverse journeys.
- **S6:** at most five evidence-led repair loops close without owner control.
- **S7:** source-blind vision and founder/human sample inspect the exact final candidate.

A failed stage stops its dependent stages unless the frozen contract permits one causal repair.

## Evidence custody

The product repository and company output paths are frozen only at activation. Counted run records name
candidate, seeds, policy version, toolchain fingerprint, receipts, metrics and disposition. Decisive
rendered evidence is content addressed; mutable aliases are convenience only.
