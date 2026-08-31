# EXP-17 frozen task portfolio specification

**Authority:** subordinate to [`EXP17_PROTOCOL.md`](EXP17_PROTOCOL.md) and the
[experiment sprint](../../../exp-sprints/exp-sprint-17-worker-architecture-benchmark.md)

The portfolio samples four economically recognisable work shapes. It is sparse by design: two
independently authored instances per shape, conditional scale runs only where the first result preserves
the decision, and no new family until the current map changes a routing decision.

Exact task bytes, fixtures and hashes are created by an independent fixture author and frozen before
the first arm. The descriptions below are contracts for those fixtures, not permission to reuse current
production defects or evaluator knowledge.

## 1. Common construction rules

Every task instance has:

- an owner-facing outcome brief with a real consumer and no architecture hint;
- exact starting bytes and permitted references;
- native acceptance checks or a locked evidence rubric;
- at least one hidden serious-defect case;
- a bounded effect and network policy;
- task-specific budget and safety envelope shared across arms;
- one independently authored sibling instance of matched estimated difficulty; and
- no solution bytes or prior run artifacts in model-visible history.

Fixture authors do not evaluate outputs. Reviewers do not see arm identity. Producers do not see hidden
cases, sibling-arm artifacts or review notes before scores freeze.

## 2. S-C — small coherent coding

These tasks measure irreducible supervisory overhead. Each should be solvable by one strong actor in a
small number of coherent edits while still requiring diagnosis and native verification.

### `SC-1` — exact callback idempotency defect

- **Artifact:** a frozen standalone service that turns a provider callback into one durable terminal
  outcome.
- **Planted behaviour:** one ordinary duplicate or reordered callback can emit a duplicate terminal
  event or overwrite a more conclusive state.
- **Outcome:** repair the defect without broadening the service API; preserve valid first delivery and
  crash/restart behaviour.
- **Visible gate:** focused unit and service tests.
- **Hidden serious cases:** duplicate after restart and reordered terminal/error delivery.
- **Consumer:** an operations engineer deciding whether the callback path can be enabled.

### `SC-2` — bounded native interaction regression

- **Artifact:** a small frozen interactive component or game control with an input-space/feedback bug.
- **Planted behaviour:** one control or interaction works in the implementation's coordinate system but
  violates the player-visible contract under a hidden orientation/focus case.
- **Outcome:** restore the intended interaction and regression coverage without redesigning the product.
- **Visible gate:** focused headless interaction test plus prepared native target.
- **Hidden serious cases:** alternate orientation/focus and repeat activation.
- **Consumer:** a product owner accepting a specific user-visible repair.

No parallel arm runs for S-C.

## 3. L-C — large coherent coding

These tasks are deliberately coupled enough that fake component splitting should lose. One actor owns
the complete artifact in both `C` and `R1`.

### `LC-1` — Swift Arrival networked delivery repair slice

- **Artifact:** an isolated frozen Swift Arrival snapshot distinct from any active EXP-16 candidate.
- **Outcome:** make one complete networked route/cargo/recovery journey pass its native host/client,
  mechanics and player-visible gates.
- **Required coupling:** movement, vehicle authority, interaction feedback, cargo custody and route
  completion all affect the same journey.
- **Visible evidence:** focused mechanics tests, exact host/client receipts and one prepared native run.
- **Hidden serious cases:** one obstruction/re-entry seed and one peer-authority/cargo seed.
- **Consumer:** a game product owner accepting a bounded playable repair, not a refactor.

### `LC-2` — durable inbound-to-work outcome

- **Artifact:** an isolated service/repository that admits a simulated business signal, creates durable
  work, survives process replacement and publishes one outcome receipt.
- **Outcome:** close the end-to-end signal path under duplicate and requirement-change conditions while
  preserving authority and secret boundaries.
- **Required coupling:** ingestion, durable state, worker execution, idempotency and terminal evidence
  share one correctness story.
- **Visible evidence:** service-level clean path and restart test.
- **Hidden serious cases:** duplicate around process death and stale work after material change.
- **Consumer:** an operator deciding whether unattended signal processing is safe to enable.

No `RP` arm runs for L-C. A worker may use ordinary local process concurrency but cannot delegate
another autonomous producer.

## 4. P-I — parallel independently closing non-coding work

These tasks measure useful capacity, not prose volume. Every unit is directly consumable and scored
locally; there is no generic model synthesis stage.

### `PI-1` — account renewal action briefs

- **Input:** frozen simulated CRM, product-usage, support and correspondence records for 24 accounts.
- **Unit output:** one concise, source-grounded renewal action brief per account: state, evidence,
  risk/opportunity, next action, owner and confidence.
- **Local gate:** required fields, source support, policy compliance, correct account isolation and blind
  account-level usefulness.
- **Serious defects:** fabricated evidence, cross-account contamination, wrong next action under policy,
  or missing high-risk account.
- **Consumer:** one named account owner per brief. No portfolio summary is required.
- **Conditional scale:** a frozen 96-account corpus with matched risk distribution.

### `PI-2` — customer-support resolution packets

- **Input:** frozen simulated customer cases, product facts and support policy for 24 cases.
- **Unit output:** one resolution packet per case: diagnosis, permitted response, internal action,
  escalation disposition and evidence.
- **Local gate:** case isolation, policy version, factual correctness, resolvability and blind support
  usefulness.
- **Serious defects:** unsafe or disallowed advice, stale-policy use after addressed change, invented
  product state, or unresolved case claimed closed.
- **Consumer:** the support operator handling that exact case. No generic synthesis is required.
- **Conditional scale:** a frozen 96-case corpus with matched severity distribution.

Arms at base scale are `C`, `R1` and `RP-Q2`. `RP-Q2` receives two disjoint unit sets and each worker
delivers locally. The lead resolves only explicit conflicts or invalid units; it does not rewrite all
outputs. If base quality remains within tolerance, run one matched `R1/RP-Q2` scale pair per task. Q4 is
outside this sprint unless Q2 saturates while preserving tail quality and a founder amendment adds it.

## 5. E-L — longitudinal changing work

These tasks measure continuity and recovery. Each runs across an initial outcome, material change,
duplicate signal, process replacement and scheduled follow-up.

### `EL-1` — continuously maintained product release

- **Artifact:** a frozen small product/site with a real release-quality brief, native checks and review
  surface.
- **Initial outcome:** produce one release candidate and evidence pack.
- **Material change:** replace one substantive requirement after useful work exists.
- **Duplicate:** resend the exact change with the same causal identity.
- **Process event:** kill the productive Codex process after a content checkpoint.
- **Follow-up:** inject one timed defect report after idle.
- **Consumer:** a product owner deciding whether the candidate remains releaseable.
- **Hidden serious cases:** stale requirement in final output and lost/unverified work after recovery.

### `EL-2` — standing market-intelligence decision ledger

- **Artifact:** a frozen simulated stream of company, competitor and market evidence feeding named
  decision slots over several cycles.
- **Initial outcome:** produce a source-backed decision ledger with explicit unknowns and actions.
- **Material change:** inject evidence that reverses one earlier conclusion.
- **Duplicate:** repeat one source/signal exactly.
- **Process event:** kill the productive Codex process after a useful checkpoint.
- **Follow-up:** deliver one scheduled source refresh after idle.
- **Consumer:** a strategy lead making the named decisions, not requesting a generic research summary.
- **Hidden serious cases:** stale conclusion survives contradictory evidence, duplicate counted twice,
  unsupported claim or lost provenance after recovery.

No `RP` arm runs for E-L. External facts are frozen/simulated so no arm benefits from search timing and
no live outreach or business mutation occurs.

## 6. Transfer tasks

Transfer measures whether useful understanding survives architecture, not whether an actor memorised a
hidden answer.

- **`TR-LC`:** one adjacent but unseen end-to-end defect in the same domain as the completed L-C
  instance, starting from the accepted artifact and only the arm's allowed persistence.
- **`TR-EL`:** one new material signal and decision request after the completed E-L event script,
  starting after an idle boundary.

Run `C` and `R1` only. Freeze transfer bytes before primary runs and reveal them only after the paired
primary scores lock.

## 7. Event equivalence

For E-L, the neutral controller watches observable milestones shared by both arms:

1. artifact checkpoint exists and focused gate has run;
2. deliver the material change;
3. after semantic acknowledgement, deliver the exact duplicate;
4. after the next useful content checkpoint, kill the named productive process group;
5. when terminal or idle according to the shared runner, advance the simulated clock and deliver the
   scheduled follow-up.

The controller records timestamps and receipts but gives no semantic rescue. If an arm never reaches a
milestone, the frozen safety envelope classifies it; the controller does not choose a more convenient
checkpoint.

## 8. Run count and stopping

Maximum primary programme:

- `C/R1`: 4 shapes × 2 instances × 2 arms = **16** runs;
- base `RP-Q2`: 2 P-I instances = **2** runs;
- conditional scale: 2 P-I tasks × `R1/RP-Q2` = **4** runs;
- transfer: L-C and E-L × `C/R1` = **4** runs;
- **maximum 26 counted runs** before any separately approved optional arm.

The expected programme is smaller because the first decisive loss stops that mechanism/cell. One valid
pair per coding and non-coding region plus the longitudinal treatment is the minimum useful evidence;
finishing all 26 is not a success criterion.

## 9. Freeze deliverables

Before the first counted run, commit privately:

- task briefs and starting-tree/corpus digests;
- visible gates and sealed hidden-fixture commitments;
- consumer-specific rubrics and serious-blocker definitions;
- per-family budgets and safety envelopes;
- balanced arm order;
- event scripts and milestone definitions;
- transfer commitments; and
- producer-leak audit proving no task solution or sibling artifact is in model-visible state.
