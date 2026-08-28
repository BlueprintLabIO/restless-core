# EXP-09 results: continuous responsibility, bounded execution

**Status:** Complete

**Disposition:** Current organisational primitives suffice after thin continuity and Runtime repairs.
No `Mission` entity, polling actor, heartbeat, workflow engine or autonomous-improvement state machine
is earned.

## Decision

A standing accountable lead can sustain useful work across changing signals while Exec remains
available. The lead should receive routine signals directly, judge whether the artifact should
change, commission bounded Staff Work only when value exists, and return to idle after each cycle.
Continuity belongs in durable responsibility, exact source facts, Work/Attempts and ordinary files or
Git. It does not require a continuously running model process.

The architecture is:

```text
owner establishes mandate once
  -> Exec appoints one accountable non-producing lead, then returns
  -> routine material signal reaches that lead
  -> lead judges no-op, bounded update, review or exception
  -> Staff produces in an isolated Attempt
  -> exact artifact and gates become terminal evidence
  -> Runtime delivers one durable terminal fact to the lead
  -> final accepted Git Work is safely promoted
  -> lead accepts, repairs, escalates or returns to idle
```

## Workload A: editorial relay versus direct responsibility

Both GPT-5.6 Sol arms maintained one article through initial publication and correction, deferred a
low-trust contradiction, rejected an irrelevant signal and stayed genuinely idle for 45 seconds.
The direct standing-lead arm removed most relay overhead:

| Measure | Owner -> Exec relay | Direct standing lead | Change |
| --- | ---: | ---: | ---: |
| Model attempts | 41 | 21 | -48.8% |
| Summed reported usage tokens | 2,209,976 | 801,082 | -63.8% |
| Estimated list cost | $21.8784 | $9.0604 | -58.6% |
| Exec wakes | 9 | 1 | -88.9% |
| Work nodes | 11 | 7 | -36.4% |
| Attempts | 13 | 7 | -46.2% |

The direct arm advances. Routine signal relay through owner and Exec adds cost, duplicate recovery
wakes and Work without improving the editorial outcome. The source injection remained a controlled
owner-to-lead message because a generic Authority external projector was not callable. This proves
the downstream topology, not zero-owner inbound autonomy.

## Workload B: continuous economic opportunity preparation

The fifth opportunity run delivered the entire frozen six-signal sequence:

| Signal | Expected behavior | Observed behavior | Result |
| --- | --- | --- | --- |
| O01 qualified inbound | Produce and independently review one dossier | Two attributed Work nodes, exact producer-to-reviewer commit, verified clean promotion | Pass |
| O02 irrelevant consumer | No Work or repository change | Lead rejected locally; zero Work and zero Git change | Pass |
| O03 duplicate delivery | No Work or repository change | Lead deduplicated locally; zero Work and zero Git change | Pass |
| O04 public price evidence | Update same dossier without review | One bounded Staff update, no reviewer, one safe promotion | Pass |
| O05 stale company fact | No Work or repository change | Lead rejected locally; zero Work and zero Git change | Pass |
| O06 high-value exception | Update and independently review an owner-ready package | Two attributed Work nodes, exact review, unsupported capabilities preserved, no effect | Package passed; owner attention deliberately deferred |

All five created Work nodes completed. There was one Exec wake, no external-effect receipt and one
clean final dossier. During the exact 45-second quiet interval there were zero new events, Work nodes,
model calls or repository changes.

The high-value package exposed a useful product distinction. “Owner-ready” can mean the package is
prepared now while owner attention remains deferred until its observable condition is met. The lead
chose that calm behavior. The hidden disposition expected owner judgement immediately, so the product
contract must explicitly distinguish `prepared for future judgement` from `owner decision owed now`.

The hidden evaluator also expected two files under `reviews/`, although the visible charter required
independent attributable review but did not require that storage shape. OrgIntel already held exact
review Work, Attempts and artifacts. Hidden tests may evaluate behavior; they must not silently require
an unbriefed representation.

## Failures that earned repairs

The invalid runs are causal evidence, not discarded setup:

| Run | Observed failure | Smallest repair |
| --- | --- | --- |
| r1 | Producer and reviewer started from sibling `main`; a cognitive integration task moved a checked-out ref and dirtied shared state | Resolve exact dependency commits and bind artifacts to the Runtime-observed full commit |
| r2 | Exact dependency lineage worked, but the lead commissioned promotion-only Work that again mutated shared state | Runtime-owned clean fast-forward promotion on the final Work |
| r3 | The lead interpreted `integration_branch` as a temporary feature branch; safe promotion correctly refused it | Give the field one meaning: an existing checked-out shared branch, normally `main`, on final Work only |
| r4 | Review revision 2 restarted from stale `main`; its hot ACP locator still named revision 1's workspace and crashed | Prefer the prior candidate commit for revisions; reconstruct sessions when the workspace changes |
| r5 | A pre-terminal progress note woke the lead before gates and final state landed; no later completion fact woke it | Runtime terminal callback after artifacts, gates and Work state; Staff does not send ceremonial status mail |
| durability audit | Live callback had a crash window between terminal state and message creation | Existing Attempt carries a tiny recoverable outbox bit; live and restart flush the fact exactly once |

These are plumbing fixes around existing concepts. They do not justify a workflow engine.

## Clean callback replay

Run r6 replayed standing-desk creation plus O01 after the callback and messaging fixes. Staff sent no
progress mail merely to wake supervision. Producer commit `b46b447` closed, the lead woke from the
Runtime terminal fact, and the reviewer started from that exact commit. Reviewer commit `5dfe29a`
included an attributable review and correction, passed the verifier, promoted `main` cleanly and
caused the final lead wake. Exec was not involved after initial delegation.

## What is now established

1. Continuous value generation is event-driven bounded work under durable responsibility, not an
   immortal agent loop.
2. Routine signals go to the nearest accountable lead. Exec owns portfolio changes, not relay.
3. No-op judgement is valuable output but should not create production Work or an artifact ledger.
4. Exact Git lineage is part of coordination truth: dependencies inherit the producer commit and
   revisions inherit the rejected candidate.
5. Integration is a bounded deterministic Runtime operation after acceptance, not cognitive Work.
6. ACP sessions are hot only within one exact workspace. A new revision reconstructs from durable
   facts instead of loading stale workspace context.
7. Supervisors need terminal facts after evidence lands. Progress prose cannot substitute for a
   durable completion callback.
8. Genuine idle is a success state. The system produced no heartbeat theater during quiet time.

## Remaining product gaps

- Generalise Authority-owned authenticated external-source projection directly to the nearest
  accountable lead. The experiment used controlled owner injection.
- Make `prepared for future owner judgement` versus `owner decision owed now` explicit in owner-handoff
  guidance and evaluate both without increasing owner noise.
- Keep review evidence outcome-native. Require a repository review file only when it is genuinely part
  of the useful artifact, not because an invisible evaluator expects one.
- Dogfood the same architecture against real inbound signals and governed effects. `_test` opportunity
  preparation proves causal behavior, not market demand or revenue.

## Verification

- Full live-Postgres OrgIntel suite passed.
- Daemon suite: 151 passed, 5 live-only tests intentionally ignored.
- Strict lint passed for OrgIntel, daemon and CLI targets.
- Daemon and CLI builds passed.
- The terminal supervisor outbox test proves one owed message, one flush and zero duplicate flush.

Machine-readable evidence is in [`results/`](results/).
