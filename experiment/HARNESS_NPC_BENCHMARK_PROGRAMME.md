# Harness → NPC → benchmark programme

**Status:** Complete; all three gated stages concluded with compact evidence

**Order:** Sprint 26 → EXP-16 → EXP-17

## Decision this programme serves

Restless needs to answer three questions in the right causal order:

1. Can unattended work execute on an exact, recoverable substrate without an operator repairing the
   harness?
2. Can that substrate support a useful embodied NPC layer that both improves Swift Arrival and makes
   game-development feedback much cheaper?
3. With substrate and game-loop confounds removed, when does Restless supervision outperform the same
   strong Codex worker acting solo?

Running the comparison first would measure current harness defects. Building throwaway test macros
before the NPC layer would reduce short-term playtest cost while creating a second motor-control stack.
Adding more agents before measuring one supervised worker would confound supervision with team size.

The programme therefore fixes the execution floor, converts repetitive play into product capability,
then measures the organisational crossover.

## Terminal programme result

Sprint 26 passed. EXP-16 produced useful mechanics and one real repair but failed the player-equivalent
S7 boundary. EXP-17 then completed four valid sparse identical-worker pairs: outcome quality was at
practical parity, while `R1` added 77%-194% latency and 74%-230% spend per cell without a recovery or
continuity win. The observed performance route is one capable worker; the accountable lead remains the
governance and exception owner. Exact disjoint `RP-Q2` throughput is the remaining high-value question.

## Stage A — exact unattended execution

**Authoritative sprint:** [Sprint 26](../docs/sprints/sprint-26.md)

Sprint 26 owns the architectural fixes that models should not have to reason around:

- exact Attempt coordinates and immutable candidate identity;
- hermetic actor workspaces and external transient caches;
- leased ports, displays, temporary directories and process groups;
- declarative, coalesced, content-keyed native gates;
- checkpointed feedback and explicit interruption;
- decision-bearing supervisory wakes;
- transactional promotion and immutable review targets; and
- crash/restart reconciliation plus deletion of manual escape hatches.

### Exit gate A

The integrated EXP-15 failure fixture passes clean, concurrent and restart variants with:

- zero model turns spent repairing the substrate;
- zero operator branch, ownership, port, process or artifact repair;
- no cross-Attempt connection, leaked child, false gate pass or half-promotion;
- exact candidate and evidence lineage after restart; and
- zero raw captures, engine caches or transient workspaces retained in Git.

Unit tests, compilation or a successful happy path do not independently satisfy this gate. The Sprint
26 run report and deletion record are the evidence authority.

## Stage B — embodied NPC playtesting and gameplay

**Authoritative experiment:** [EXP-16](exp-sprints/exp-sprint-16-embodied-npc-playtesting.md)

EXP-16 applies the exact substrate to the frozen Swift Arrival candidate. It builds one embodied-agent
architecture shared by:

- a player-equivalent evaluator;
- a deterministic golden-path and recovery driver;
- a production robber; and
- a production vampire passenger.

The layer performs motor control through ordinary game actions and physics. Language-and-vision models
judge sparse semantic moments and outcomes; they do not steer frame by frame.

### Exit gate B

EXP-16 must establish all of the following before the benchmark starts:

- the evaluator completes and recovers through player-legal actions on frozen and held-out seeds;
- low-level model control decisions fall by at least 90% against the frozen baseline;
- robber and vampire roles reuse the same perception, belief, goal and action contracts;
- accepted product repairs pass discovery and held-out regression scenarios;
- bot-pass versus blind-player-fail cases remain visible; and
- the final exact candidate, compact evidence and raw-artifact cleanup are frozen.

Failure is useful if it identifies the boundary honestly. EXP-16 need not produce a positive NPC result
for EXP-17 to proceed, but it must leave a stable game-development loop and no active harness confound.

## Stage C — identical-worker architecture benchmark

**Authoritative experiment:** [EXP-17](exp-sprints/exp-sprint-17-worker-architecture-benchmark.md)

EXP-17 compares:

- `C`: one Codex actor owns the complete outcome;
- `R1`: Restless Exec delegates to a non-producing lead supervising one identical Codex worker; and
- `RP`: the same Restless topology with extra identical workers only for predeclared, independently
  closing units.

The producing model, reasoning effort, first-party Codex session protocol, task tools, starting bytes,
aggregate task budget and native gates are identical. Restless coordination and durable supervision are
the treatment. Hidden chain-of-thought is neither requested nor treated as evidence; configured effort,
messages, tool calls, checkpoints, receipts, cost and outcome are observable.

### Exit gate C

The programme closes with a crossover guide, not a universal winner:

- solo Codex for cells where supervision adds only overhead;
- one supervised Codex worker where continuity, recovery or evidence discipline repays that overhead;
- selective parallel workers where units close locally and throughput improves without quality loss;
- `unknown` where the sparse evidence is not decisive.

At least one coding and one non-coding cell, plus the longitudinal change/recovery treatment, must reach
a valid paired decision. Every conclusion names its model, tools, work shape and uncertainty.

## Global invariants

These hold across all three stages:

1. Exec always delegates an executable request to an accountable lead and returns to availability.
2. A lead remains a non-producing supervisor, including when it supervises only one worker.
3. One end-to-end worker is the shared-outcome default; more workers require independently useful
   units and measured available demand.
4. Runtime correctness is deterministic. Judgement remains model-owned and reviewable.
5. Completion is event- and evidence-driven. Time limits are safety envelopes, never success signals.
6. Specific fan-in is allowed only for a named consumer outcome; generic synthesis is not free work.
7. Exact model, effort, tools, candidate and evaluator lineage are frozen before counted calls.
8. Raw captures, model homes, engine caches, temporary worktrees and process state are operating data,
   not research artifacts. Compact evidence survives; junk is deleted.
9. Provider, substrate, model, coordination, product and evaluation failures are attributed separately.
10. No stage broadens into a general agent framework merely to keep a hypothesis alive.

## Programme ledger

| Stage | Contract | Activation dependency | Terminal artifact |
| --- | --- | --- | --- |
| A | Sprint 26 | none | passing integrated run report + deletion record |
| B | EXP-16 | Gate A + exact frozen EXP-15 candidate | NPC contract, run ledger, findings, Dogfood 4 decision |
| C | EXP-17 | Gate A + stable EXP-16 closure + Codex parity preflight | paired ledger, blind scores, crossover guide, keep/change/purge decisions |

Approval of this programme does not bypass any stage budget, external-effect boundary, model admission
probe or exact activation freeze in the linked contracts.
