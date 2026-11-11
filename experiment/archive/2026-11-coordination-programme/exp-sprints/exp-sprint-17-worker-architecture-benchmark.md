# EXP-17 — Codex solo versus Restless-supervised Codex

**Status:** Concluded after four valid sparse first pairs; `C` selected in all observed performance cells,
with `RP-Q2` deferred to a separately frozen continuation

**Primary decision:** For which work shapes does a non-producing Restless supervisor earn its overhead
over the same Codex worker operating solo, and where does selective parallel staffing earn a further
advantage?

**Primary arms:** Codex solo (`C`) · Restless with one Codex worker (`R1`) · selectively parallel
Restless (`RP`) only in predeclared separable cells

**Producing model:** exact `gpt-5.6-sol`, `high` reasoning effort, through the same pinned first-party
Codex session runtime in every counted primary arm; exact admission must pass before freeze

**Optional arm:** Hermes (`H`) only after exact model, effort, tool and custody parity passes

**Authority:** Isolated private artifacts and configured model spend only; no external sends, writes,
deployment, purchase or live-company mutation

## Why this experiment exists

Earlier experiments established useful local principles: one end-to-end worker is usually better than
a mechanically decomposed team for one coherent artifact; independent workers create throughput when
units close locally; a non-producing lead can protect mission alignment, recovery and continuous work;
and Exec must delegate to leads so several departments can operate concurrently while Exec remains
available.

They did **not** establish that Restless beats a current strong Codex actor. Recent Swift Arrival work
used other worker routes and was confounded by substrate defects. Restless can supervise exactly one
Codex worker, so the honest comparison is not “team of weaker agents versus Codex.” It is:

> the same capable worker, task and tools, with or without durable supervision—and, only where work is
> independently separable, with selective parallel capacity.

The output is a crossover guide by work shape, not a universal winner or a marketing benchmark.

## Executable specification set

Three subordinate contracts remove the remaining implementation ambiguity:

- [`CODEX_PARITY_SPEC.md`](../coordination/experiments/EXP-17/CODEX_PARITY_SPEC.md) — one shared
  first-party Codex runner, exact model/effort/tool/context parity, session recovery and observable
  effort/process evidence;
- [`TASK_PORTFOLIO_SPEC.md`](../coordination/experiments/EXP-17/TASK_PORTFOLIO_SPEC.md) — concrete
  small/large coding, locally closing account/support and longitudinal product/research tasks; and
- [`EVALUATION_SPEC.md`](../coordination/experiments/EXP-17/EVALUATION_SPEC.md) — blinded custody,
  rubrics, pair validity, run ledger and crossover decisions.

This sprint remains authoritative. A subordinate specification can narrow execution but cannot add an
arm, effect, task family or budget.

## Work packages

| Package | Outcome | Exit evidence |
| --- | --- | --- |
| `P0 Runner parity` | identical Codex production runtime in solo and Restless arms | parity manifest + passed no-count probe |
| `P1 Freeze` | task, fixture, order, budget, rubric and hidden commitments | digests + leak audit |
| `P2 First pairs` | one valid `C/R1` pair in S-C, L-C, P-I and E-L | locked blind scores + pair-validity rows |
| `P3 Sparse decision` | continue or stop each cell by predeclared gate | decision ledger |
| `P4 Parallel` | `RP-Q2` only on locally closing P-I units | unit/tail quality + throughput including lead work |
| `P5 Recovery/transfer` | equivalent process/change treatment and unseen follow-ups | event and transfer reports |
| `P6 Reveal` | outcome scores frozen before process identity/economics | sealed mapping opened after signatures |
| `P7 Route` | scoped operating defaults | crossover guide + keep/change/purge |

## Hypotheses

- **H1 — small coherent work:** `C` will usually match quality with lower latency and cost because the
  supervisory value has little time to accrue.
- **H2 — large coherent work:** `R1` may earn its overhead through alignment, evidence discipline,
  failure recovery and protection against premature completion, even though one worker still produces.
- **H3 — separable throughput:** `RP` will win elapsed time or completed throughput only when units have
  local closure and require no broad semantic fan-in.
- **H4 — continuous/changeful work:** `R1` will preserve mission, candidate lineage and useful work
  across events, interruption and process replacement better than an episodic solo actor.
- **H5 — no free average:** architecture effects will interact with size, coupling, ambiguity and event
  horizon strongly enough that one pooled “best system” label is misleading.

Any hypothesis may fail. Supervisor distinctness is product doctrine, but performance, cost and
appropriate routing remain empirical.

## Arms

### C — Codex solo

One Codex actor receives the frozen owner brief, exact starting inputs, native tools, total budget and
acceptance contract. It may plan, inspect, implement and verify. It has no Restless Exec, lead, Staff
handoff, durable Work/Attempt service or supervisor intervention. It may use ordinary Git checkpoints
and task-local notes available equally to the worker in other arms.

### R1 — Restless-supervised Codex

Exec delegates the complete executable outcome to one accountable non-producing lead and returns to
availability. The lead commissions exactly one Codex Staff worker, supervises evidence and recovery,
and never edits the artifact. Sprint 26 owns exact execution, gates and promotion. The total task budget
includes Exec/lead overhead.

### RP — selectively parallel Restless

The same topology as `R1`, except the lead may commission multiple Codex workers only for partitions
declared independently useful before the run. Each owns exact units with local acceptance and direct
delivery. A synthesis worker is forbidden unless the task contract names one specific integrated
outcome that cannot close locally.

`RP` does not run on coupled cells. This is not an all-tasks “team” arm.

### H — optional Hermes

Hermes enters causal comparison only if preflight proves the exact Codex model/version, reasoning
effort, context, tool permissions, starting bytes, aggregate budget, output custody and gate access.
If parity is impossible, a Hermes run may be reported descriptively in a separate appendix but cannot
change the primary conclusion or substitute for a counted arm.

## Fairness and frozen controls

Before the first counted run, freeze:

- exact task bytes, starting commit/data/corpus and hidden fixtures;
- exact model route, version and reasoning effort for every producing Codex actor;
- tools, network policy, runtime image, environment and native gate definitions;
- one aggregate spend/token/time safety ceiling per task and one owner-attention policy;
- evaluator rubric, practical-difference thresholds and tie rules;
- balanced arm order and opaque artifact labels; and
- task instances and seeds, including held-out transfer cases.

The primary comparison holds **aggregate task budget** equal. This intentionally counts supervision as
overhead: granting `R1` the solo worker budget plus free lead spend would not measure efficiency. Report
worker and supervisory spend separately so later product routing can choose a different service level.

Completion is event-driven. A generous wall-clock limit is a common safety envelope and failure class,
not evidence that work is complete or correct.

## Sparse task portfolio

Use four work families with two independently authored, held-out instances each. Instances match on
estimated native difficulty but do not share solution bytes.

| Cell | Work shape | Example class | Primary arms | Why it discriminates |
| --- | --- | --- | --- | --- |
| **S-C** | Small, coherent, low ambiguity | bounded defect with native regression | C / R1 | measures irreducible supervisory overhead |
| **L-C** | Large, coherent, high coupling | integrated game/product slice or architectural repair | C / R1 | tests alignment and recovery without fake parallelism |
| **P-I** | Broad, independently closing units | frozen-corpus research/account/operations batch | C / R1 / RP | tests throughput without mandatory fan-in |
| **E-L** | Longitudinal, changing requirements | multi-cycle product or operational artifact with signals | C / R1 | tests durable mission and process replacement |

At least one coding and one non-coding family are included. Non-coding outputs must be economically
recognisable—such as account briefs, research decisions, campaign assets, customer-operation cases or
market monitoring—but operate on frozen/simulated inputs and perform no live outreach or external
mutation.

### Non-coding local-closure rule

For `P-I`, each unit must be directly useful to its named consumer without a generic “combine all
answers” stage. Valid examples include one accepted account brief per account, one resolved support case
per case, one monitored entity alert per entity and one source-backed research claim per predeclared
decision slot. Fan-in is allowed only for a specific decision that logically consumes several units;
it is measured as work, not assumed free.

## Counted run matrix

The maximum programme before sequential stopping is:

- `C` and `R1`: 4 families × 2 instances = **8 runs per arm**;
- base `RP-Q2`: the same 2 independently separable `P-I` instances = **2 runs**;
- conditional scale: one matched `R1/RP-Q2` scale pair for each `P-I` task = **4 runs**;
- transfer: one related unseen follow-up for `L-C` and `E-L` in `C` and `R1` = **4 runs**.

Maximum primary programme: **26 counted runs**—16 `C/R1` primary runs, two base `RP-Q2` runs, four
conditional matched `R1/RP-Q2` scale runs and four `C/R1` transfer runs. Sequential stopping should
make the realised programme smaller. Hermes mirrors a frozen subset only after its parity gate and
separate founder budget approval.

Use sequential stopping:

1. Run one balanced pair in each family.
2. Stop a mechanism in that family after a decisive quality loss or invalidity.
3. Run the second pair only when the first result leaves the routing decision live.
4. Run matched `R1/RP-Q2` scale variants only if the base `RP-Q2` result preserves quality.
5. Do not add more task families until the predeclared cells yield a crossover guide or an explicit
   ambiguity worth resolving.

## Longitudinal and recovery treatment

The `E-L` instances contain a frozen event script:

1. initial outcome request;
2. one material requirement change after useful work exists;
3. one exact duplicate/no-op signal;
4. one killed productive process after a checkpoint; and
5. one scheduled follow-up after an idle interval.

Both arms receive the same events at equivalent observable checkpoints. After process replacement,
`C` resumes through the shared Codex runner using only its allowed thread/session, Git and neutral
task-local persistence. `R1` uses the same Codex resume semantics plus normal Work/Attempt recovery and
lead supervision. Neither receives hidden rescue from the controller.

## Evaluation

### Outcome judgement

- Artifact labels reveal neither arm nor process.
- Native tests/receipts run first where available.
- Two source-blind reviewers score the locked rubric independently. They see the owner brief, artifact,
  allowed evidence and known constraints—not transcripts, cost, arm identity or prior scores.
- A disagreement beyond the locked tolerance triggers one named adjudicator judging the specific
  disputed criterion. There is no general committee synthesis.
- Process identity and efficiency are revealed only after outcome scores freeze.

### Primary measures

1. blind outcome quality on the task-specific locked rubric;
2. completion without owner/controller rescue; and
3. serious defects or false completion at the native gate.

### Secondary measures

- elapsed and active time;
- total and producer-only model cost/tokens;
- model turns, tool calls and failed/repeated actions;
- supervisor wakes, interventions and decisions changed;
- handoff/context reconstruction loss;
- gate executions, exact cache reuse and duplicated verification;
- time to recover from change/process death and useful work retained;
- owner-attention requests, regressions and artifact-lineage errors; and
- transfer-task quality, time and relevant retained learning.

Report every metric by cell and arm. Use paired differences for matched instances; do not hide a small-
task loss behind a high-throughput win.

## Predeclared practical decisions

Rubric scores are normalised to 100. A difference smaller than 5 points is treated as practical quality
parity unless the score hides a serious blocker or safety/evidence failure.

- **R1 earns the default for a cell** when it remains within 5 points of `C`, introduces no additional
  blocker, and materially improves at least one of: rescue-free completion, recovery/retention,
  regression rate or transfer quality. Its latency/cost overhead remains visible.
- **C remains the default for a cell** when quality is at parity and `R1` adds at least 20% latency or
  spend without a material recovery, continuity or quality benefit.
- **RP earns use for a separable cell** only when quality remains within 5 points of the better single-
  worker arm and elapsed time or completed throughput improves at least 30%, including any specific
  fan-in cost.
- **A serious native blocker outranks average rubric score.** One arm cannot “win quality” while failing
  the outcome's non-negotiable gate.
- **No universal default is inferred** from a pooled average. The terminal artifact is a routing table
  over observed work shapes plus explicit unknown regions.

These thresholds are product decisions, not significance claims. Report raw paired results and
uncertainty; do not imply statistical generality from two instances.

## Failure attribution

Every failure receives exactly one primary class before arm identities are compared:

- `product/outcome-failure`;
- `model/actor-failure`;
- `coordination-failure`;
- `runtime/harness-failure`;
- `provider/capacity-failure`;
- `evaluation-infrastructure-invalid`; or
- `authority/safety-stop`.

Sprint 26 defects invalidate the affected pair and permit one repaired replay from both arms. A product
loss is counted, not repaired until it disappears.

## Proposed budget and safety envelope

- Primary `C/R1/RP` programme ceiling: **USD 300**, subject to founder approval at activation.
- After the no-count calibration, freeze a per-arm ceiling for each task family. Every compared arm
  receives the same aggregate task ceiling; `R1/RP` must pay Exec/lead/extra-worker overhead from it.
  An arm cannot borrow from its sibling to rescue a loss. At least USD 40 of the aggregate remains
  reserved for valid symmetric replays and transfer/conditional-scale tasks.
- Hermes requires a separate ceiling and founder amendment after parity; it cannot consume the primary
  reserve.
- Every task receives the same generous safety envelope, frozen from parity preflight and proportional
  to its family. An envelope kills leaked/stalled execution and classifies it; it never means success.
- A stopped branch returns unspent budget. Completing 24 runs is not a reason to override a sequential
  stop rule.

## Acceptance criteria

EXP-17 closes only when:

1. all counted artifacts have exact input/model/tool/budget lineage;
2. every primary comparison uses the same producing Codex model and effort;
3. arm order is balanced and blind scores freeze before process reveal;
4. at least one coding and one non-coding family reach a valid paired decision;
5. the longitudinal treatment runs the same change, duplicate, kill and scheduled event in both arms;
6. `RP` is tested only on independently closing units and its fan-in work is explicit;
7. owner/controller rescue, invalid pairs and infrastructure retries are fully reported;
8. the result publishes per-cell crossover decisions, not a pooled winner; and
9. concrete Restless keep/change/purge and default-routing updates follow from the evidence.

## Stop conditions

Stop the affected branch for:

- inability to prove model/effort/tool/start parity;
- cross-arm artifact, transcript, rubric or held-out-case leakage;
- three repeated substrate-invalid pairs after Sprint 26;
- live external effects or an unapproved budget expansion;
- a lead producing artifact work in `R1/RP`;
- `RP` decomposing a semantically coupled artifact merely to exercise a team; or
- evaluator knowledge of arm identity before scores freeze.

## Required deliverables

- frozen tasks, fixtures, order, budgets, rubrics and hashes;
- one machine-readable run ledger and pair-validity record;
- blind outcome scores and adjudications;
- cost/time/turn/recovery/transfer metrics;
- per-family findings with explicit confounds;
- `CROSSOVER_GUIDE.md` mapping observed work shapes to `C`, `R1`, `RP` or unknown;
- Restless keep/change/purge decisions and follow-up tickets; and
- a short founder-facing result that states where supervision paid and where it did not.

## What this experiment cannot conclude

It cannot establish that all agent organisations beat all solo agents, that Codex is the best possible
worker, that two task instances represent an industry, or that supervision's distinct governance value
is unnecessary when it costs more. It can decide the next evidence-backed routing defaults for the
tested models, tools and work shapes.
