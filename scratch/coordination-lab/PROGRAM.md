# Coordination harness evidence programme

Status: completed scratch experiment, not production architecture
Started: 2026-08-22
Completed: 2026-08-22
Result: twenty evidence-producing mini-sprints, followed by the purge and recommendation in [`FINAL_REPORT.md`](./FINAL_REPORT.md)

## Question

Can Restless get materially better company coordination from a thin first-party ACP harness composed
from Pi, seven generic OrgIntel commands, and a few strong but overridable workplace rules—without
turning the Runtime Bridge into a planner or turning OrgIntel into a workflow engine?

The programme separates three concerns:

1. **Harness:** exact launch inputs, model/tool execution, event streaming, cancellation, and truthful
   terminal state.
2. **OrgIntel:** ordinary mutable organisational state: Actor, Goal, Work, Attempt, edges, Message,
   Decision, Schedule, handoff, artifact reference, and operational event.
3. **Team behaviour:** judgement exercised through seven commands: `send`, `commission`, `redirect`,
   `report`, `request_judgement`, `decide`, and `schedule`.

The harness does not own teams, Work, dependencies, company memory, planning, or authority. OrgIntel
does not own the model loop, shell, files, Git, or token stream.

## Fixed scenario

The real input is the creature-collecting browser-game mandate in the Cosmon company config. Runs
start from the same observed Git seed (`514b7b3d0a65e093af608b08ca142344412181f4`) and must improve a
runnable artifact rather than merely produce a plan. The full mandate deliberately exceeds a short
run: the experiment measures whether the company frames useful Work, delegates, converges, verifies,
and leaves a truthful continuation state under uncertainty.

Every comparable run records:

- exact seed and mandate hash;
- exact harness commit/hash and launch contract;
- exact OpenRouter model IDs and zero-price proof captured immediately before launch;
- wall-clock and turn limits;
- injected failures;
- resulting Work/Attempt graph and chronological runtime events;
- Git artifacts and external executable checks;
- score breakdown and evidence locators.

Generated run state lives below ignored `runs/`; durable findings and scorecards live in this folder.

## Comparison modes

- **A — single agent:** one model process may inspect, implement, and verify. This tests raw model and
  harness ability without coordination.
- **B — loose team:** Exec can start role-labelled workers and exchange prose, but does not receive
  explicit Work/Attempt/edge semantics.
- **C — OrgIntel:** the same harness and model pool use the seven commands and explicit Work,
  Attempt, dependency, revision, report, decision, and wake semantics.

A, B, and C use the same success contract and executable checks. A mechanism is not credited merely
because its model is stronger or its run is longer.

## Mini-sprint protocol

Each mini-sprint is one bounded empirical loop:

1. Name the dominant observed failure or uncertainty from the prior evidence.
2. Classify it: harness, coordination semantics, team behaviour, model capability, evaluation, or
   scenario difficulty.
3. Name one intervention and the mechanism by which it should change the outcome.
4. State risks and dispositions: accepted, guarded, pending fix, or invariant.
5. Run deterministic conformance/fault checks first.
6. Run one live free-model experiment against the fixed scenario or an explicitly named focused
   probe derived from it.
7. Score only observable evidence using `SCORECARD.md`.
8. Record the result, surprise, remaining bottleneck, and retain/revert/purge decision.
9. If two consecutive interventions fail to move the relevant dimension, stop tuning that lever and
   branch to a structural alternative.

Versions are earned by runs. Empty design revisions do not count. The tentative frontier below is a
hypothesis, not a precommitted roadmap; evidence may reorder or replace it.

| Range | Tentative focus |
| --- | --- |
| v01-v03 | First-party ACP/Pi conformance and A/B/C baselines |
| v04-v07 | Exact launch control, bounded turns, quiescence, and callbacks |
| v08-v10 | Dependencies, convergence, review, and model/role diversity |
| v11-v14 | Crash, cancellation, stale input, duplicate event, and unknown outcome recovery |
| v15-v17 | Context focus, repeated learning, and adaptive team shape |
| v18-v19 | Harder scenario and deliberate simplification/purge |
| v20 | Best retained design versus fixed baselines and final audit |

## Free-model policy

Every new live inference goes through OpenRouter and uses a model whose live `/api/v1/models` entry
has both `pricing.prompt == "0"` and `pricing.completion == "0"` immediately before launch. The model
must accept text and advertise tool support. The run records the returned catalogue facts.

Pinned model IDs are preferred. `openrouter/free` is a last-resort availability probe because its
dynamic routing weakens reproducibility. A model becoming paid or disappearing stops that launch; it
does not silently fall back to a paid model. Provider credentials are read at process launch and are
never copied into run artifacts, prompts, child shells, or model-visible context.

The initial diverse pool is selected from the live free collection:

- an orchestration/reasoning model;
- a coding-specialist model;
- a smaller/fast model used to expose instruction and protocol fragility;
- a second independent model for critique where availability permits.

The pool is refreshed during the programme because free availability is volatile.

## Stop and 10x rules

The programme pauses local optimisation when any of these occurs:

- the same failure class dominates two consecutive mini-sprints;
- coordination machinery grows while accepted executable output does not;
- score rises only through more turns, context, or workers;
- the harness starts deciding Work shape or company policy;
- deterministic infrastructure work dominates model/coordination learning;
- model variance is being mistaken for a mechanism improvement.

The response is to identify the system constraint and test the smallest structural change that could
remove it across scenarios. Examples include replacing polling with event wakes, replacing prose
handoffs with artifact references, making termination truthful, or deleting a coordinator feature
that the agents can handle through ordinary state and judgement.

## Evidence standard

Agent narration is not proof. Evidence priority is:

1. runnable artifact and user-path probe;
2. independent executable checks;
3. repository/file/commit inspection;
4. OrgIntel state and chronological harness events;
5. agent report, used only as a claim to verify.

A run that crashes or times out is still evidence. Unknown remains unknown; the scorer never infers a
successful effect, artifact, or completion from an interrupted tool call.
