# EXP-16 — Embodied NPC playtesting and production-behaviour frontier

**Status:** Concluded negative at S7; mechanics policies passed, rendered player-equivalent role coverage did not

**Controls:** Dogfood 4 persistent Swift Arrival charter · EXP-15 final frozen candidate and metrics

**Primary question:** Can one reusable embodied-agent architecture make ordinary game-development
feedback roughly an order of magnitude cheaper and faster while also becoming the foundation for real
driver, robber and vampire NPCs?

**Models:** exact GPT-5.6 Sol for production and GPT-5.6 Terra for sparse semantic/visual judgement,
subject to route/effort admission at preflight. No model may drive frame-by-frame.

**Authority:** Local private build and configured model spend only

## Why this experiment exists

The current loop asks a language-and-vision model to perform low-level embodied control through sparse
observations. That is useful for legibility and semantic judgement, but wasteful for steering, timing,
collision recovery and repetition. Swift Arrival also needs NPCs that can drive, pursue, shoot, rob a
truck and inhabit transported vampire roles. A throwaway golden-path macro would reduce test cost but
create no product value.

The candidate is therefore a shared embodied-agent stack: deterministic enough for repeatable tests,
expressive enough for product behaviour, and constrained to the same meaningful game actions as a
player. Vision remains a critic of moments and outcomes rather than a substitute for motor control.

## Decision to test

Use one layered NPC architecture for two profiles:

1. **Player-equivalent evaluator.** Receives only player-observable/local sensors and acts through the
   same movement, vehicle, interaction, collision and weapon interfaces available to a player. It may
   not teleport, directly mutate mission/cargo state or read hidden outcome flags.
2. **Host-authoritative production NPC.** May use server-owned perception and beliefs appropriate to an
   in-world actor, but reaches the world through the same locomotion, vehicle, interaction and combat
   action adapters. It cannot directly award deliveries, steal cargo or set mission completion.

Both use:

`perception -> belief/blackboard -> goal/utility selection -> behaviour tree/planner -> action adapter -> physics/authority`

The architecture is accepted only if the evaluator profile materially tightens the development loop
and the production profiles create repeatable, meaningful gameplay without a second incompatible AI
stack.

## Native outcome

From the exact EXP-15 candidate, the experiment produces:

- a reusable embodied NPC body and action contract;
- a golden-path delivery driver that completes the canonical workday through real game interactions;
- a chaos/recovery driver that exposes and attempts common failure states;
- one robber encounter using pursuit, weapon and cargo-theft behaviour; and
- one vampire passenger role whose state changes its cooperation or interference.

The autonomous loop runs mechanics continuously, requests sparse rendered review at semantic moments,
turns grounded findings into bounded Product Work and proves each retained repair against unchanged
seeds plus fresh held-out seeds.

## Executable specification set

The experiment's implementation and counted evidence are constrained by three subordinate contracts:

- [`NPC_ARCHITECTURE_SPEC.md`](../coordination/experiments/EXP-16/NPC_ARCHITECTURE_SPEC.md) — shared
  body, policy profiles, action authority, receipts, progress/recovery and anti-cheat boundary;
- [`SCENARIO_SPEC.md`](../coordination/experiments/EXP-16/SCENARIO_SPEC.md) — baseline, conformance,
  delivery, recovery, robber, vampire, blind-validation and cleanup matrices; and
- [`EVIDENCE_SPEC.md`](../coordination/experiments/EXP-16/EVIDENCE_SPEC.md) — frozen manifest, run
  ledger, metrics, blind rubric, repair evidence and interpretation rules.

This sprint remains authoritative. A subordinate document may make execution more exact but cannot
broaden roles, budget, effects or acceptance.

## Work packages

| Package | Product-owned outcome | Exit evidence |
| --- | --- | --- |
| `P0 Freeze` | exact candidate, baseline and hidden commitments | frozen manifest + two reproduced baseline cases |
| `P1 Body` | shared observations, blackboard, goals, actions and receipts | BODY-01..10 + CHEAT-01..07 |
| `P2 Delivery` | complete player-legal workday evaluator | visible and held-out DEL matrix |
| `P3 Recovery` | bounded diagnosis and recovery | REC matrix + useful terminal packets |
| `P4 Robber` | fair pursuit/combat/cargo-theft encounter | ROB-01..06 |
| `P5 Vampire` | cooperative and adverse passenger behaviour | VAM-01..06 |
| `P6 Loop` | at most five evidence-led product repairs | replay + held-out regressions per accepted repair |
| `P7 Review` | independent exact-candidate judgement | locked blind review + founder/human sample |
| `P8 Close` | honest result and clean operating state | G1–G5 verdicts + cleanup proof |

Packages close in order except that small robber/vampire scaffolds may be inspected after P1. A later
package cannot turn an earlier failed gate into a pass by adding content.

## Frozen architecture contract

### Perception

- Player-equivalent sensors expose local transforms, collision/contact, visible/interactable targets,
  current controls feedback, carried cargo, vehicle state and audio/event cues only when the player
  could perceive their equivalent.
- Production sensors may add host-authoritative sight/hearing and role knowledge, but not future route,
  evaluator-only truth or direct mission mutation.
- Every observation records source tick, actor, visibility basis and seed.

### Belief and goal selection

- A compact blackboard holds observed—not magically true—facts, current goal, last progress and bounded
  memory needed for recovery.
- Utility selects among explicit goals; a small behaviour tree or bounded planner executes them.
- Long stalls, oscillation and repeated action failure emit typed events rather than spinning.

### Action and authority

- Actions are semantic but physical: move/steer/look, enter/exit, interact, carry/drop/strap, aim/fire,
  board, pursue, retreat and wait.
- Action adapters drive the existing host-authoritative game contracts. Test-only direct mutation is
  forbidden in counted runs.
- Every action produces a bounded receipt: start/end ticks, parameters, observable result and failure.

### Determinism and review

- Scenario, NPC policy and environment use separately recorded seeds.
- Headless mechanics and event traces run cheaply and often.
- Rendered vision is event-triggered at ambiguity, regression, stage transition and final review. It
  judges legibility, coherence and visible outcome; it does not micromanage controls.

## Experimental stages

### Stage 0 — freeze the comparison baseline

- Sprint 26 integrated acceptance passes.
- Freeze exact EXP-15 candidate, canonical preset/seeds, native commands and current vision-agent
  baseline: completion, elapsed time, model decisions, cost, interventions and blocker rate.
- Reproduce at least one known success and one known failure without changing the game.

### Stage 1 — shared embodied body

- Implement perception, blackboard, goal selection, action adapter, receipts and stall detection.
- Prove walk, collide, interact, enter, drive, exit, carry/drop and recover on small scenes.
- Add anti-cheat tests that fail on teleport, direct mission mutation, hidden-state reads and actions
  outside the ordinary authority path.

### Stage 2 — golden-path delivery driver

- Complete depot selection/loading, truck traversal, route driving, destination unload and result.
- Run the canonical workday on a fixed seed set, then held-out seeds.
- Keep failures as product evidence; do not script around broken collision, targeting or feedback.

### Stage 3 — chaos and recovery driver

- Inject missed interaction, blocked door, dropped cargo, wrong turn, vehicle obstruction, early exit,
  failed pickup, re-entry and recoverable cargo displacement.
- Detect stall/oscillation and choose bounded recoveries through normal actions.
- Produce compact failure packets suitable for one repair decision.

### Stage 4 — robber production role

- Add perceptible ambush, pursue, aim/fire, board, steal/carry cargo and retreat behaviours.
- Scope only the minimum weapon/damage/perception primitives required for one coherent encounter.
- Prove both player success and player failure/recovery paths without direct cargo-state mutation.

### Stage 5 — vampire passenger production role

- Model at least calm, agitated/hungry and escape/interference states.
- React to observable conditions such as harsh driving, light, damage, delay or nearby threat.
- Produce one cooperative and one adverse but recoverable journey. Dialogue may express state but does
  not replace behaviour.

### Stage 6 — continuous product loop

- Run NPC mechanics on an event-driven schedule, not a polling model loop.
- Escalate only semantic uncertainty or a compact blocker to a non-producing Game Product lead.
- The lead commissions one bounded Staff repair; unchanged seeds prove the fix and held-out seeds test
  overfit.
- Sparse Terra review samples decisive rendered moments and the final player-visible journey.

### Stage 7 — blind validation and decision

- Run fresh evaluator seeds and independent vision review with no producer history.
- Sample a real founder/human journey before any claim that NPC success equals player acceptance.
- Publish the speed/quality frontier and decide which NPC capabilities enter Dogfood 4.

## Counted design

Use a sequential sparse design rather than a large factorial:

- **Baseline:** at least 6 frozen journeys using the current sparse vision-driving loop: 3 canonical,
  3 recovery.
- **Evaluator:** the same 6 seeds plus 6 held-out seeds after Stage 3.
- **Production roles:** at least 6 robber encounters and 6 vampire journeys, balanced across canonical
  and adverse seeds.
- **Continuous repair:** at most 5 genuine NPC-discovered Product Work loops. Infrastructure retries do
  not count.

Do not expand a stage after a decisive failure. Repair the common embodied substrate only when evidence
shows the failure is architectural; game defects become bounded product loops.

## Acceptance gates

### G1 — substrate validity

- Exact candidate, gate, process and review lineage is enforced by Sprint 26.
- NPC actions use ordinary physics/authority; anti-cheat gates pass.
- Runs are deterministic under fixed seeds and honestly variable under held-out seeds.

### G2 — evaluator utility

- Golden-path delivery completes all producer-visible scenarios and at least five of six held-out
  scenarios. Every recovery class terminates boundedly and at least six of eight held-out placements
  recover to objective completion, with no manual control or direct state mutation.
- The evaluator detects blocked/stalled/oscillating states and emits a bounded failure packet rather
  than consuming unbounded ticks.
- Compared with the frozen vision-driving baseline, low-level model action decisions fall by at least
  90%, while elapsed time and model cost improve materially. The exact measured ratio is reported; a
  “10x” claim is made only for metrics that actually cross it.

### G3 — product transfer

- Robber and vampire profiles reuse the same perception/belief/goal/action contracts.
- Each role produces repeatable, player-visible pressure, a fair success path and a recoverable adverse
  path.
- No role completes its fiction by directly mutating mission, cargo, delivery or combat outcomes.

### G4 — product-learning quality

- At least three NPC findings are independently classified as real product defects or meaningful tuning
  opportunities, or the honest negative result is that the evaluator adds no useful discovery.
- Every accepted repair passes its discovery seed and held-out regression seeds.
- False assurance is measured: bot-pass/human-or-vision-fail cases remain explicit.

### G5 — current-candidate experience

- A source-blind Terra review and one founder/human sample inspect the exact final candidate.
- NPC completion is evidence of mechanics and robustness, not proof of fun or human legibility.
- Dogfood 4 receives a concrete keep/revise/purge decision for the shared NPC stack and each role.

## Organisation

- Exec delegates once to one non-producing Game Product lead and returns to availability.
- One end-to-end GPT-5.6 Sol Gameplay Staff worker owns the shared body and production implementation
  through the frozen admitted worker harness by default. EXP-16 does not claim first-party Codex
  parity; EXP-17 owns that implementation and comparison.
- A separate source-blind player/critic owns independent evaluation and cannot edit the product.
- Add another producer only for an already stable, independently verifiable seam. Team topology is not
  the independent variable in EXP-16.
- Lead intervenes on blocker, policy, evidence conflict or terminal result, not progress events.

## Metrics

Record per run:

- exact candidate, presets and all seeds;
- simulator ticks, real elapsed time and active control decisions;
- model calls, tokens, spend and vision frames;
- goal transitions, action failures, stalls, oscillations and recoveries;
- objective completion and route/cargo/combat state receipts;
- manual interventions and infrastructure retries;
- product defects found, accepted, rejected and regressed; and
- bot-versus-blind-player agreement.

Report medians and ranges by scenario. Do not pool robber, vampire, delivery and recovery runs into one
headline success percentage.

## Proposed budget and safety envelope

- Aggregate model ceiling: **USD 120**, subject to founder approval at activation.
- At most USD 20 is spent reproducing the baseline before the architecture gate.
- At most USD 60 is spent on shared-body and production-role work, USD 25 on sparse blind vision, and
  USD 15 remains a repair reserve. Unused allocation is not a target.
- No repair silently crosses USD 8; the lead narrows or stops first.
- Long-running mechanics are event-driven. A per-process safety envelope is derived from the preflight
  baseline and terminates leaks; it is never used as completion or product evidence.
- At most 12 decisive rendered frames survive a counted semantic review, and raw captures are deleted
  after synthesis.

## Stop conditions

Stop a stage for:

- any direct mission/cargo/outcome mutation or evaluator hidden-state leak;
- three repeated instances of the same harness defect after Sprint 26;
- unbounded action loops, leaked game processes or ambiguous candidate identity;
- model control returning to frame-by-frame driving;
- a proposed general AI framework broader than the three tested roles require;
- unapproved spend/effect or raw capture accumulation; or
- a failed gate that no longer discriminates the architecture decision.

## Evidence and terminal artifacts

Commit compact specifications, source and final findings. Company-local outputs contain:

- `BASELINE.jsonl` and `RUNS.jsonl`;
- `NPC_CONTRACT.md` and `ANTI_CHEAT_RESULTS.md`;
- `FINDINGS.md`, `REPAIRS.jsonl` and `METRICS.json`;
- exact candidate and immutable review-target manifests; and
- `RESULTS.md` with gate-by-gate verdict and Dogfood 4 decision.

Raw PNG/video, engine caches and transient traces are deleted after compact synthesis. The experiment
ends as `accepted`, `provisional-loss`, `product-invalid`, `evaluation-infrastructure-invalid` or
`inconclusive`; “NPCs work” is not a valid unqualified result.

## Relationship to Dogfood 4

Dogfood 4 remains the persistent Swift Arrival charter. EXP-16 governs a proposed v0.6 embodied-agent
layer and returns evidence to that charter; it does not create a new dogfood number or silently promote
experimental source.
