# EXP-16 scenario and fault matrix

**Authority:** subordinate to [`EXP16_PROTOCOL.md`](EXP16_PROTOCOL.md) and the
[experiment sprint](../../../exp-sprints/exp-sprint-16-embodied-npc-playtesting.md)

This document freezes the scenario *shape*. Exact source tree, seed values, build fingerprint and hidden
fixture hashes enter the activation manifest before a counted model call. Held-out seeds and fault
placements remain unavailable to producers until their relevant policy bytes are frozen.

## 1. Common run rules

Every counted scenario starts from an isolated exact candidate and records separately:

- scenario seed: layout, actors, cargo and perturbation;
- world seed: nondeterministic environment choices permitted by the fixture;
- policy seed: deterministic policy tie-breaking;
- profile and policy version;
- controlling and authoritative peers; and
- native gate, build and environment fingerprints.

A run is invalid, rather than a product loss, if candidate identity is ambiguous, a process/resource
lease crosses runs, required evidence is missing, a provider fails before any product action, or an
evaluator sees held-out data early. A crash caused by the candidate is a counted product loss.

No scenario is silently retried. One infrastructure-invalid run may be replayed from exact inputs after
the substrate defect is fixed and logged. Product failures remain in the ledger.

## 2. S0 baseline reproduction

| ID | Starting condition | Purpose | Required result |
| --- | --- | --- | --- |
| `B-SUCCESS-1` | frozen EXP-15 mechanics-success coordinates | prove the baseline can still succeed | reproduce exact native success receipts |
| `B-FAIL-1` | frozen public-surface/player-invalid coordinates | preserve the known failure | reproduce the failure or document exact causal drift |
| `B-VISION-C1..3` | canonical delivery journeys | current sparse vision-driving baseline | model decisions, frames, time, spend, completion |
| `B-VISION-R1..3` | recovery journeys | current sparse vision-driving baseline | same measures plus interventions and blocker class |

The baseline freezes before NPC implementation. If either known case cannot reproduce, Stage 1 does not
start until the discrepancy is classified; the team cannot repair the baseline away.

## 3. S1 shared-body conformance scenes

These are small mechanics fixtures, not evidence that the game is playable.

| ID | Capability | Perturbation | Pass evidence |
| --- | --- | --- | --- |
| `BODY-01` | walk/orient/stop | approach from off-axis | reaches tolerance without wall penetration |
| `BODY-02` | collision | direct obstacle | blocked receipt then legal reorientation |
| `BODY-03` | target/interact | one invalid distractor | selects visible valid target and receives feedback |
| `BODY-04` | enter/exit vehicle | seat initially occluded | reacquires entry point and changes seat through authority |
| `BODY-05` | drive/park | bend plus narrow stop zone | reaches and parks without transform mutation |
| `BODY-06` | carry/drop/place | cargo offset and rotated | custody and placement change through normal interaction |
| `BODY-07` | strap/unstrap | one occupied slot | respects capacity and feedback |
| `BODY-08` | aim/fire/cease | moving legal target | host confirms bounded weapon/damage receipts |
| `BODY-09` | recover | temporary blocked path | progress resumes through bounded ladder |
| `BODY-10` | fail honestly | permanent obstruction | emits terminal failure packet without spin |

`BODY-01..10` run at least once in player-equivalent mode. Applicable actions also run once in a
networked host/client configuration before production-role acceptance.

## 4. Anti-cheat fault injections

Each fault is planted in a disposable fixture and must make the governed run fail:

| ID | Forbidden fault | Expected gate |
| --- | --- | --- |
| `CHEAT-01` | direct actor or vehicle transform write | movement-authority violation |
| `CHEAT-02` | direct mission/delivery completion write | outcome-mutation violation |
| `CHEAT-03` | direct cargo-custody mutation | cargo-authority violation |
| `CHEAT-04` | evaluator reads hidden destination/fixture flag | observation-scope violation |
| `CHEAT-05` | action without observation/receipt lineage | receipt-lineage violation |
| `CHEAT-06` | unbounded retry loop | progress-bound violation |
| `CHEAT-07` | client claims unconfirmed material result | host-authority violation |

The clean implementation must then pass the same suite. Static search alone does not satisfy these
tests.

## 5. S2 delivery evaluator matrix

Freeze six producer-visible scenarios and commit six held-out scenarios from the same strata.

| Stratum | Visible IDs | Held-out IDs | Required variation |
| --- | --- | --- | --- |
| clean delivery | `DEL-C1`, `DEL-C2` | `DEL-HC1`, `DEL-HC2` | depot/destination choice, approach direction |
| cargo variation | `DEL-G1`, `DEL-G2` | `DEL-HG1`, `DEL-HG2` | cargo location, size/slot choice within supported contract |
| route variation | `DEL-R1`, `DEL-R2` | `DEL-HR1`, `DEL-HR2` | route turn sequence and parking orientation |

Each run must obtain work, load, board, traverse, park, unload and reach the ordinary result path. A
fixture may not preselect a hidden optimal target for the policy. The scenario receipt records completion,
ticks, action decisions, stalls, model calls and direct state mutations.

Stage 2 passes when all producer-visible scenarios and at least five of six held-out scenarios complete
without manual control or forbidden authority, with the remaining failure bounded and classified. The
full raw result remains visible; this threshold is not a claim of human playability.

## 6. S3 adverse recovery matrix

The recovery suite uses at least one visible and one held-out placement for each class:

| ID family | Injected condition | Valid recovery examples | Terminal evidence |
| --- | --- | --- | --- |
| `REC-INT` | missed/rejected interaction | reacquire, reorient, retry once | accepted action or bounded rejection |
| `REC-DOOR` | blocked door/entry | reverse, reposition, alternate legal approach | progress or blocked packet |
| `REC-CARGO` | dropped/displaced cargo | reacquire visible cargo, carry/replace | custody/placement receipt |
| `REC-TURN` | wrong turn | detect route divergence, safe correction | route progress resumes |
| `REC-VEH` | vehicle obstruction | stop, reverse/reposition, alternate path | no collision bypass |
| `REC-EXIT` | early/accidental exit | safely re-enter or continue on foot if contract permits | seat/goal transition |
| `REC-PICK` | failed pickup/occupied slot | inspect feedback, choose valid target/slot | no hidden capacity read |
| `REC-OSC` | geometry inducing repeated actions | detect signature and terminate/recover | oscillation receipt within bound |

Stage 3 passes when every class terminates within its frozen action/progress budget, no run spins or
cheats, and at least six of eight held-out placements recover to objective completion. Non-recovered
cases must produce useful failure packets.

## 7. S4 robber encounter matrix

Run six counted encounters after policy freeze:

| ID | Situation | Player/evaluator objective | Robber objective | Required evidence |
| --- | --- | --- | --- | --- |
| `ROB-01` | visible roadside warning | evade without cargo loss | pressure then disengage | readable onset and fair evasion |
| `ROB-02` | moving pursuit | reach safety | maintain pursuit/attack | legal driving, aim and damage |
| `ROB-03` | forced stop | protect/recover cargo | board and acquire cargo | ordinary boarding/custody path |
| `ROB-04` | robber gains cargo | pursue or recover | retreat with cargo | recoverable adverse path |
| `ROB-05` | obstructed ambush | exploit cover/route | replan approach | no hidden route/teleport |
| `ROB-06` | networked encounter | survive or recover | coherent host-authoritative sequence | peer/authority receipts |

Balance parameters so at least two runs favour player success, two permit robber success, and two test
recovery or ambiguity. Acceptance requires both sides to have a mechanically possible success path,
the threat to be perceptible in blind review, and every cargo/combat result to follow ordinary authority.

## 8. S5 vampire passenger matrix

Run six counted journeys:

| ID | Initial state / stimulus | Expected behaviour | Player recovery path |
| --- | --- | --- | --- |
| `VAM-01` | calm, ordinary journey | boards, remains cooperative, exits normally | normal delivery |
| `VAM-02` | harsh driving | escalating visible agitation | smooth driving/de-escalation |
| `VAM-03` | delay/hunger | complaints then bounded interference | resume progress or address declared need |
| `VAM-04` | damaging light exposure | seeks cover/safety | alter route/stop safely |
| `VAM-05` | nearby robber threat | reacts to threat according to state | protect/escape/de-escalate |
| `VAM-06` | adverse networked journey | attempts escape or interference | regain cooperation or complete recoverably |

Acceptance requires one cooperative and one adverse-but-recoverable full journey, visible state
transitions beyond dialogue, and no direct mission or passenger-outcome mutation.

## 9. S6 evidence-led repair loops

At most five genuine NPC-discovered product findings may enter repair:

1. lock the failure packet and classify it as game, NPC, harness or evaluation;
2. the non-producing lead decides whether it is decision-relevant;
3. one Staff worker receives one bounded repair outcome;
4. rerun the exact discovery scenario;
5. run all relevant unchanged scenarios plus at least two held-out regressions; and
6. accept, revise once or reject. A second broad repair is new Work and consumes another loop.

The experiment does not manufacture five repairs. If no material defect is found, it records the honest
negative result.

## 10. S7 blind validation

- Select exact final candidate and content-addressed review target before evaluator launch.
- Run at least two fresh delivery/recovery journeys, one robber encounter and one vampire journey using
  seeds hidden from producers.
- A source-blind GPT-5.6 vision reviewer sees only the player-visible outcome and locked rubric.
- One founder/human journey samples controls, legibility and fun after model scores freeze.
- Any bot-pass/human-fail is reported as false assurance and blocks a human-playability claim.

## 11. Cleanup gate

After compact synthesis, the controller proves:

- no raw PNG, video or frame sequence remains in the product or Restless repositories;
- no disposable model home, imported engine cache, attempt worktree or capture directory remains;
- no game/server/capture process or resource lease remains live; and
- retained evidence consists only of source, compact JSONL/JSON/Markdown, content-addressed decisive
  evidence allowed by protocol, and exact commit/digest manifests.

Cleanup failure keeps the experiment open even if product gates pass.
