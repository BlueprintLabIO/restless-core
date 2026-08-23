# Ordinary-team crossover programme

Status: active; precedes wildcard screening

## Questions

1. When does an ordinary lead-plus-worker arrangement beat the same strong lead working alone?
2. Is the crossover explained by work size, potential parallelism, coupling, work type, lead-alone
   saturation or worker capability?
3. Where does a critic add information without the integration cost of another producer?
4. When does adding a second or later worker create positive marginal value?

Every arm begins after the company Exec has delegated the executable request to one accountable team
lead and returned to availability. The experiment changes organisation below the lead only.

## Baselines

- **B0 — lead alone:** one strong accountable lead owns production, integration, proof and completion.
- **B1 — ordinary team:** the same lead may commission exactly one worker through a normal bounded
  artifact brief, receives no shared-history wildcard, and owns integration and completion.
- **B2 — critic:** B0 followed by a fresh artifact-only critic with producer reasoning withheld.

B0 and B1 map the producer-team frontier. B2 is tested on a smaller subset where hidden error or
subjective quality is material; it does not replace the B0/B1 comparison.

## Three meanings of parallelism

- **Potential:** work that could proceed independently given stable seams and acceptance targets.
- **Realised:** useful actor work that actually overlapped without blocking or duplication.
- **Beneficial:** realised overlap whose value exceeded briefing, communication, integration, rework
  and additional error cost.

Do not call a workload parallelisable merely because a lead created several Work nodes.

## Pre-run workload features

Two fresh judges complete `templates/workload.md` from the frozen success contract and starting
artifact before any arm runs. Disagreements are retained and reconciled as judgement, not averaged into
false precision.

The primary variables are:

- domain and native artifact type;
- lead-alone pilot time and tool-call range;
- independently acceptable artifact seams;
- dependency width and critical-path depth;
- shared mutable-state surface;
- interface stability;
- independent verifiability;
- specialist diversity;
- tool/external latency that can genuinely overlap;
- breadth uncertainty versus whole-artifact coherence;
- cost of detecting and repairing a bad contribution.

Post-run observation corrects the prediction: accepted contributions without lead modification,
useful actor overlap, blocked time, interface churn, integration diff, redundant reads and rework are
recorded separately.

## Sparse calibration design

Start with two domains and the four most discriminating corners, then one centre point:

| Cell suffix | Size | Potential parallelism / coupling |
|---|---|---|
| **SL** | small | low parallelism / high coupling |
| **SH** | small | high parallelism / low coupling |
| **LL** | large | low parallelism / high coupling |
| **LH** | large | high parallelism / low coupling |
| **MM** | medium | mixed seams and one integration point |

Initial domains:

- **C — coding/product:** executable and visually reviewable Cosmon increments;
- **R — sourced research:** a frozen inspectable source corpus and decision memo, with no synthetic
  market fact entering a live company.

This creates ten cells. Each receives one matched B0/B1 pair: twenty initial counted runs. Arm order is
randomised within each cell. B0 pilot evidence may recalibrate a workload's realised size before the
paired result is interpreted, but the scenario is never silently edited.

The existing v23 pair is historical evidence for medium-to-large tightly coupled coding. It is not a
substitute for B1 because that team arm used two producers plus a critic.

## Sequential selection

After the ten cells:

1. Replicate surprising reversals immediately.
2. Add points between clear wins and losses to locate the boundary.
3. Repeat only cells whose uncertainty could change routing.
4. Test B2 on cells with judgement-heavy acceptance or hidden defects.
5. Increase team size only where B1 beat B0.
6. Vary worker strength/provider only in cells close to the crossover.
7. Add document/design and operational sentinel cells only after the structural variables have a
   plausible explanation.

This is sequential sparse coverage, not a claim that untested regions behave like tested ones.

## Measurements

### Outcome gate

- native artifact acceptance;
- deterministic/external evidence where enumerable;
- blind severity-ranked review where judged;
- no speed win credited to a materially worse outcome.

### Time and economics

- request → lead ownership;
- time to first useful artifact and accepted outcome;
- wall time and summed model time;
- lead, worker, critic, coordination and cached tokens;
- tool calls, compute and nominal cost even when provider price is zero;
- owner attention and Exec occupied time.

### Parallelism and coordination

- useful actor-time overlap;
- total useful work / observed critical-path work;
- worker time blocked on lead/upstream state;
- contributions accepted without lead modification;
- communication and clarification turns;
- repeated discovery and duplicate work;
- interface changes after delegation;
- lead integration, conflict and repair effort;
- harness/provider failures separated from organisational failures.

The economic question is:

```text
value of overlapped accepted work
− briefing
− communication
− integration
− rework
− duplicated computation
= net team gain
```

## Escalating team size

For a B1 win only:

```text
lead alone → lead + 1 → lead + 2 → lead + 4 only if +2 has positive marginal value
```

Later workers must receive additional independent seams; total scope, budget and acceptance contract
remain comparable. A larger team does not earn credit for doing a larger task.

## Output

The crossover programme produces scoped curves and a routing explanation across size, coupling and
domain. It must not collapse into a universal “delegate above N minutes” threshold. Wildcards begin
only after an ordinary team loses in a region with credible unused parallel value or another observed
communication bottleneck.
