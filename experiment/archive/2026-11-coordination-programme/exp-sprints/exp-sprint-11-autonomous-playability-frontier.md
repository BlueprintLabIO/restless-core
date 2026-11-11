# Experiment Sprint 11 - Autonomous playability frontier

**Status:** Complete — `product-judgement-failure`

**Decision owner:** Founder

**Date:** 28 August 2026

**Controlled product charter:** Swift Arrival Dogfood 4

## Decision this sprint must produce

Determine whether Restless can turn Swift Arrival from a technically verified network-interaction
prototype into a genuinely playable vertical slice through repeated autonomous diagnosis, build,
native playtest and revision cycles without the owner managing a backlog.

EXP-11 controls and independently referees Dogfood 4. It does not replace the dogfood charter, create
a second product direction or treat evaluator approval as founder acceptance. Dogfood 4 remains the
source of product intent. EXP-11 owns the isolated test, frozen controls, evaluator admission,
evidence lineage, measurements and final experimental disposition.

## Why this test is now necessary

EXP-10 proved that a standing lead could supervise several useful product cycles, suppress an exact
duplicate, recover a killed worker, handle a direct scheduled review and remain quiet. It did not
prove that the resulting game was playable.

The founder's first ordinary use exposed two foundational faults that the prior technical evidence
missed:

- `A` and `D` felt reversed relative to the camera; and
- truck geometry appeared solid but did not provide real collision behavior.

This is the next frontier: not whether autonomous work continues, but whether it finds, prioritizes
and repairs the product problems that matter to an actual player.

## Hypothesis

> One standing non-producing Game Product lead, supervising one end-to-end gameplay worker by
> default, can use independent native playtest evidence from `zai/glm-5.3-flash` to converge on a
> playable vertical slice while remaining evidence-driven, quiet between events and aligned with the
> product charter.

The contrary result is that the company optimizes executable probes instead of experience, needs the
owner to decompose the work, cannot obtain trustworthy playtest evidence, churns through cosmetic
changes, or declares success while foundational interaction defects remain.

## Owner-authorised evaluator amendment — 28 August 2026

Wave 0 proved that the frozen `zai/glm-5.3-flash` route was infrastructure-invalid, and the attempted
OpenRouter route subsequently failed its credential-capacity probe. The founder explicitly replaced
the unavailable evaluator with GPT-5.6 Sol, routed as exact selector `litellm/gpt-5.6-sol` through
OMP's generic OpenAI-compatible adapter and the already provisioned local gateway (`GPT_BASE_URL`
plus `GPT_API_KEY`). This amendment does not relabel any
fallback as Flash or erase the failed admission result. From this point onward, references to a fresh
Flash referee in the execution loop and completion criteria mean a fresh, exact GPT-5.6 Sol vision
referee; the final disposition must retain `supported-with-evaluator-limit` if the product evidence
otherwise supports the hypothesis. Producer and referee sessions remain isolated, and all image,
native-input, recapture, withholding and evidence-lineage requirements remain unchanged.

## Control relationship with Dogfood 4

| Responsibility | Accountable party |
| --- | --- |
| Product purpose and intended experience | Dogfood 4 charter |
| Product diagnosis, priority and acceptance into the candidate | Game Product lead |
| Source, tests, native build and implementation evidence | Gameplay Staff |
| Independent native use and observations | GLM-5.3 Flash Vision referee |
| Experimental controls, lineage and metrics | EXP-11 harness |
| Final judgement of playability and taste | Founder |

The experiment may stop an invalid run, reject contaminated evidence and prevent promotion. It may
not direct the implementation backlog, silently change the outcome contract or tell the referee what
the producing team expected to happen.

## Frozen run

Run one isolated company named `swift_arrival_playability_r1_test` with no external effects and a USD
50 total model ceiling. Reserve USD 35 for production and USD 15 for independent playtesting as
measurement envelopes; the company-level limit remains the enforceable backstop.

The production organisation is:

1. Owner gives Exec one outcome mandate.
2. Exec delegates it to one accountable Game Product lead and returns to availability.
3. The lead remains a supervisor and commissions one end-to-end gameplay Staff worker by default.
4. The lead may add a specialist only after citing an observed seam that one worker cannot efficiently
   close. Team size is an evidence-based choice, not a target.

The production route is frozen at T0 after a live provider check, with `zai/glm-5.3` as the expected
model. The referee is the exact selector `zai/glm-5.3-flash`, admitted separately under the contract
below. The producer and referee must not share a session.

The current Dogfood 4 v0.4 work and commit `84ff1745b29267708599e94036ec6f7a2a7e0457` are only a baseline
candidate. T0 freezes an executable baseline only after the source repository has a clean exact commit
and tree, the pending v0.4 work is intentionally included or excluded, and fresh native and network
probes pass. Uncommitted runtime state is never copied as an experimental baseline.

## Wave 0 - admit the referee or stop

Before product work begins, prove that `zai/glm-5.3-flash` can act as a real native playtester through
the first-party model route. Admission requires all of the following:

1. the exact model selector reaches successful inference with recorded non-zero usage;
2. a known screenshot calibration pair is ingested and correctly distinguished;
3. the model can cause bounded real input in the native Godot window and inspect a fresh capture after
   that input;
4. one short exploratory session produces a durable report with an action trace, screenshot locators,
   observed state and uncertainty; and
5. no source, diff, producer reasoning, expected failure list or scripted answer is present in its
   context.

If image ingestion, native control, capture, durable output or exact routing fails, record
`infrastructure-invalid` and stop. A scripted replay, another model, a text-only transcript or human
description may diagnose the infrastructure but cannot be counted as the requested Flash playtest.

## Starting evidence

The only owner-supplied product signal after the frozen charter is:

> Horizontal movement feels reversed relative to the view, and apparently solid truck geometry does
> not behave like real collision geometry.

This signal is evidence, not a decomposed backlog. The lead must diagnose causes, choose the smallest
coherent next slice and decide whether control space, character physics or another foundation comes
first. After this initial signal, the experiment supplies no curated issue list. Further production
cycles must arise from the artifact, deterministic checks, Flash playtests or the lead's own review.

## Event-driven autonomous loop

1. A fresh, responsibility-scoped Flash session receives only the player goal, controls and native
   candidate.
2. It explores the real client and emits a durable playtest report under the referee contract.
3. The report reaches the accountable lead as untrusted but material evidence with a stable source
   key.
4. The lead corroborates or rejects observations, chooses the highest-leverage product change and
   commissions bounded Staff Work.
5. Staff owns source, tests, native execution evidence and an exact clean candidate commit.
6. Deterministic positive and negative mechanics probes run against that candidate.
7. A new fresh Flash session receives the native candidate without the producer's context.
8. The lead accepts, revises or stops from the combined evidence.

A candidate artifact wakes the referee; a referee result wakes the lead. No cron, heartbeat or
recurring “keep improving” prompt is allowed. A one-shot schedule is permitted only for a genuine
delayed observation. Quiet periods are expected and must be free of speculative work.

## Frozen playability contract

The run aims for one compact vertical slice, not a content-rich game. The candidate must provide:

1. **View-relative control:** `W`, `A`, `S` and `D` consistently move in the direction implied by the
   current camera, on foot and while driving, with an observable calibration check.
2. **Real physical boundaries:** the player cannot traverse truck walls, floor or closed geometry;
   the doorway remains usable; ground and carried-object interactions do not rely on boundary clamps
   masquerading as collision.
3. **Coherent first-person feel:** mouse look, movement response, blocked motion and camera behavior
   are understandable and avoid obvious jitter, clipping or inversion.
4. **Legible interaction:** targeting, pickup, drop, seat entry, seat exit and unloading expose enough
   feedback for a new player to form the right action model.
5. **Real multiplayer state:** host and client preserve authoritative, mutually visible player,
   vehicle and delivery state rather than presenting two unrelated local illusions.
6. **Recoverable delivery loop:** ordinary mistakes such as dropping outside the zone, leaving the
   seat or approaching an interaction from an imperfect angle do not cause a soft lock.
7. **Coherent world behavior:** the truck, route, passenger and cargo remain meaningfully present and
   do not contradict the delivery premise.
8. **Honest presentation:** raw debug output cannot dominate the experience or imply success that the
   underlying state has not earned.
9. **Executable integrity:** positive and negative probes pass from the exact clean candidate, and the
   client and host shut down cleanly.
10. **Independent usability:** two consecutive fresh, withheld-context Flash sessions can complete the
    loop or produce correct, evidence-backed descriptions of a remaining product blocker.

Deterministic checks establish mechanics, not feel. Flash establishes independent agent-use evidence,
not fun. The founder alone supplies the final product and taste judgement.

## Referee boundary

The Flash referee may:

- receive a concise player goal and public controls;
- inspect the rendered native client;
- operate bounded player input and recapture resulting state;
- explore beyond the happy path; and
- report observations, reproduction steps, uncertainty and a verdict.

It may not:

- inspect source, diffs, tests, Work, producer messages or expected defects;
- edit product files or prescribe implementation;
- author or reorder the product backlog;
- approve promotion or declare founder acceptance; or
- reuse a prior candidate's private reasoning as hidden continuity.

Every candidate gets a fresh session. The report is evidence for the lead, not an instruction to the
worker. See the full [`playtest-referee-contract`](../coordination/experiments/EXP-11/playtest-referee-contract.md).

## Success contract

1. T0 records a clean exact source commit and tree plus fresh baseline native and multiplayer results.
2. Wave 0 admits the exact Flash selector with real image ingestion, native input, recapture and
   durable evidence. No fallback is counted.
3. Exec dispatches once and remains available; one standing non-producing lead supervises all product
   cycles.
4. The owner supplies no implementation decomposition or issue backlog after the starting evidence.
5. Every accepted change is Staff-attributable through Work, Attempt, exact commit, gates and lead
   judgement.
6. The lead addresses foundational interaction risk before cosmetic expansion and records why each
   priority changed the playable outcome.
7. At least one accepted material cycle originates from a defect not named in the starting owner
   signal, demonstrating autonomous discovery rather than backlog execution alone.
8. Rejected or mistaken referee observations remain visible and are resolved with native evidence,
   not silently discarded.
9. The frozen playability contract passes, including two consecutive fresh Flash sessions, without
   weakening criteria during the run.
10. A prepared founder session runs against the exact final candidate and ends in `accept`, `revise`
    or `reject`; model agreement is never represented as founder taste.
11. No external effect, public deployment, purchase, outreach or product-source promotion occurs.
12. The result selects one disposition: `supported`, `supported-with-evaluator-limit`,
    `manual-backlog-still-required`, `product-judgement-failure`, `infrastructure-invalid`, or
    `inconclusive`.

## Measures

- playability rubric movement by exact candidate;
- source of each material defect: owner, Flash, lead, deterministic probe or regression;
- accepted, revised, rejected and duplicate product cycles;
- foundational versus cosmetic priority choices;
- false completion or “playable” claims contradicted by later native evidence;
- time from material evidence to clean accepted candidate;
- Work, Attempts, model calls, tokens and estimated spend by lead, Staff and referee;
- owner interventions and owner minutes after the initial mandate;
- native action traces, screenshot evidence and reproduction success;
- mechanics regressions and recovery from ordinary player mistakes;
- quiet-period Work, model, event and repository deltas; and
- harness-caused, evaluator-caused and product-caused failures.

No single autonomy score may hide these measures. The primary outcome is a playable artifact with
traceable independent discovery and no owner-managed backlog.

## Stop rules

The maximum is ten reviewed candidate cycles, not a target. Stop earlier when:

- the frozen success contract and founder review are ready;
- the USD 50 total model ceiling is reached;
- the exact Flash referee loses admission or cannot produce trustworthy native evidence;
- two consecutive accepted cycles show no material playability improvement;
- the lead identifies a stable blocker requiring new authority, unavailable infrastructure or a
  product-direction decision; or
- there is no new evidence and the correct state is quiet.

Elapsed time bounds observation only. Semantic evidence decides whether a cycle is complete.

## Failure and validity rules

- A Flash self-report without image, action and resulting-state evidence is not a playtest.
- Replaying the same capture or report does not create a second independent judgement.
- Local scripted input remains a mechanics oracle; it cannot stand in for exploratory play.
- A passing evaluator does not erase an executable failure, and a passing probe does not erase a
  coherent native-use failure.
- The lead may reject an evaluator claim only with a recorded reproduction or contradictory native
  evidence.
- The experiment cannot inject synthetic product defects. It measures discovery and prioritisation on
  the real artifact.
- Outcome criteria are frozen at T0. The team may clarify evidence collection but may not make the
  contract easier after observing results.
- Experimental commits remain in the isolated company until the founder explicitly chooses
  promotion into Dogfood 4.

## Terminal result — 28 August 2026

The autonomous production organisation materially improved the game from baseline `84ff1745` to
experimental candidate `41f4fa53`. The final candidate passed all five exact deterministic Work gates,
including positive delivery, route-zero shortcut rejection, seat re-entry, movement/collision and
cargo recovery.

It failed the independent-usability contract. The terminal strict R19 referee directly proved the
shortcut rejection, then used a fresh native launch to pick up, deliberately drop and recover the
parcel, drive the loaded truck to the visible route end, exit, re-enter, move again, exit again and
attempt delivery in the destination structure. No delivery completion appeared. The independent lead
and Exec both recorded a reproducible post-route-40 blocker.

The required two consecutive fresh completions were not achieved, and founder acceptance review was
withheld. The result is therefore `product-judgement-failure`, not supported or
`supported-with-evaluator-limit`. The GPT-5.6 Sol amendment remains a limitation, and the unavailable
GLM-5.3 Flash route is not relabelled.

Recorded spend across the production and replacement referee companies was USD 170.887523. Each
company stayed within its own ceiling, but the original USD 50 total ceiling was not enforced across
companies. Continued execution was owner-authorised, yet the missing aggregate guard is a material
harness defect and makes the evaluation loop too expensive for routine use.

See [`RESULTS.md`](../coordination/experiments/EXP-11/RESULTS.md),
[`FRICTIONS.md`](../coordination/experiments/EXP-11/FRICTIONS.md), and
[`FOUNDER_REVIEW.md`](../coordination/experiments/EXP-11/FOUNDER_REVIEW.md).

## Key risks

| Risk | Treatment |
| --- | --- |
| Flash invents observations | Require native action trace, before/after captures, uncertainty and lead corroboration |
| Producer teaches the referee the answer | Separate sessions and context; expose only player goal, controls and candidate |
| The team optimises a scripted route | Permit exploratory actions and ordinary mistakes; withhold producer test scripts |
| A fixed cycle count manufactures churn | Treat ten as a ceiling; every cycle requires new material evidence |
| Dirty Dogfood state contaminates attribution | Freeze a clean exact commit and tree before cloning |
| A vision critic is mislabeled as a playtester | Require actual input plus post-action observation in Wave 0 |
| Agent agreement is mistaken for fun | Preserve founder review as the final product decision |
| Rubric gaming replaces product judgement | Keep criteria outcome-level; lead chooses implementation and Flash explores freely |

## Frozen deliverables

1. Clean frozen Dogfood 4 baseline record and isolated company lineage.
2. Exact GLM-5.3 Flash Vision admission evidence or an infrastructure-invalid stop report.
3. One durable playtest report and native evidence bundle per reviewed candidate.
4. Per-cycle Work, Attempt, gate, Git, decision, spend and quiet-state evidence.
5. Final native candidate plus positive and negative mechanics results.
6. Prepared founder review using the exact candidate.
7. Narrative result, machine-readable metrics, friction dispositions and architectural decision.
8. A Dogfood 4 amendment and canon updates only after founder judgement and only for supported claims.

Item 6 was correctly not delivered because independent usability failed. The other terminal records
are present under `experiment/coordination/experiments/EXP-11/`.

## Approval boundary used

The approved contract authorised only the isolated EXP-11 run and minimal harness repair needed to
make the frozen referee contract executable. It did not authorise public deployment, live external
effects, promotion into Dogfood 4, a recurring autonomous worker or unbounded product expansion. No
such action occurred.
