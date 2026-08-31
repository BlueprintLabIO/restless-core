# EXP-11 execution record

**Status:** Complete; final candidate returned for revision
**Date:** 28 August 2026

This directory will execute
[`exp-sprint-11-autonomous-playability-frontier.md`](../../../exp-sprints/exp-sprint-11-autonomous-playability-frontier.md)
as an isolated controller and referee for Swift Arrival Dogfood 4.

## Question

Can one standing supervisor-led product team use independent native vision playtests to turn a
technically verified prototype into a playable vertical slice without an owner-managed backlog?

The frozen Flash route failed admission. The founder-authorised replacement is exact
`litellm/gpt-5.6-sol`; this evaluator change remains a reported limitation rather than being relabelled
as Flash evidence.

## Separation of roles

- Dogfood 4 owns the product purpose.
- The Game Product lead owns diagnosis, priority and candidate acceptance.
- Gameplay Staff owns implementation and executable evidence.
- exact `litellm/gpt-5.6-sol` owns fresh, withheld-context native playtest observations after the
  recorded evaluator amendment.
- EXP-11 owns controls, evaluator admission, evidence lineage and measurement.
- The founder owns the final judgement.

The referee is outside the production organisation. It receives the player goal, public controls and
native candidate, not source code, expected defects or producer reasoning. Its output is untrusted
evidence routed to the accountable lead; it cannot edit the game or approve completion.

## Frozen execution gates

The run required these gates before production evidence could count:

1. founder approves the sprint contract;
2. Dogfood 4 has a clean, exact and intentionally selected baseline commit and tree;
3. fresh native and network probes pass at that baseline;
4. the production model route succeeds; and
5. the exact Flash selector passes image, native-control, recapture and durable-report admission under
   the [`playtest-referee-contract`](playtest-referee-contract.md).

The exact frozen baseline was commit `84ff1745b29267708599e94036ec6f7a2a7e0457`, tree
`25733ead6eb7c83221048d323838a5cadc2a235e`.

## Planned causal sequence

1. freeze and verify the exact baseline;
2. admit the exact Flash referee or stop infrastructure-invalid;
3. dispatch one owner mandate through Exec to the standing Game Product lead;
4. deliver the founder's control/collision observation without decomposing it;
5. run a fresh blind native baseline playtest;
6. let the lead commission the highest-leverage bounded Staff work;
7. run deterministic checks and a fresh blind native playtest on the clean candidate;
8. continue only when new evidence warrants another cycle;
9. stop at contract satisfaction, a stable blocker, invalid evidence, diminishing quality or budget;
10. prepare the exact final candidate for founder `accept`, `revise` or `reject`.

No cron or heartbeat drives this sequence. Candidate artifacts wake the referee, referee reports wake
the lead, and no new evidence means no new work.

## Current result set

- `RUN-LOG.md` records the chronological production, evaluator and harness evidence.
- `RESULTS.md` records the terminal `product-judgement-failure` disposition and its decision value.
- `FRICTIONS.md` separates product, evaluator and harness failures, including the repairs already
  made and the remaining work.
- `metrics.json` is the compact machine-readable result, including the USD 170.887523 aggregate
  spend that the original per-company ceilings failed to constrain.
- `FOUNDER_REVIEW.md` records why the correct founder decision is `revise` and why hands-on acceptance
  review is withheld.
- `results/candidate-41f4fa5/` is the frozen final candidate copy with exact hashes in `RESULTS.md`.
- `results/referee-r19/` is the terminal strict two-Work clean-room evidence bundle. It proves the
  route-zero shortcut rejection and a reproducible post-route-40 completion blocker.
- `results/referee-r19/runtime-summary.json` preserves the compact terminal Work, Attempt, actor,
  spend, verdict and runtime state without requiring a live daemon.
- `results/referee-r8/` is the first valid exact-Sol clean-room playtest bundle for candidate
  `a1d96fe4ce7280501f3d193fc3c3058a23e8a914`; it returned revise because delivery was not completed.
- `results/referee-r5-invalid/` preserves a long native session that reproduced the interaction
  ambiguity but became invalid after the Runtime window died during a quota/transport interruption.
- `results/referee-r7-invalid/` preserves the active-window capture contradiction that led to exact
  numeric-window capture.

## Terminal answer

The team autonomously improved the product and discovered defects not supplied by the owner, but it
did not converge to the frozen independent-playability bar. Candidate `41f4fa53` passed five exact
deterministic gates and a production-side native review. The stronger final independent player still
failed to complete delivery after the visible route end, even after re-entry and a second exit.

The experiment is complete with disposition `product-judgement-failure`. Do not promote the
experimental candidate. The next move, if authorised, is one bounded route-end exit/unload repair
followed by one strict replication under an aggregate budget.
