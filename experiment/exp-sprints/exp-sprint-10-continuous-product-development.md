# Experiment Sprint 10 - Continuous product development

**Status:** Active

**Decision owner:** Founder

**Date:** 28 August 2026

**Live artifact:** Swift Arrival Dogfood 4, baseline commit `3dc502ae9938dafccd26286e0d28ee8f50dc60c1`

## Decision this sprint must produce

Determine whether Restless can improve one real playable product across several bounded cycles without
the owner repeatedly decomposing or restarting the work, while remaining quiet when a scheduled review
finds no decision-changing evidence.

This is the first product-development application of EXP-09's standing-responsibility result. It tests
whether the existing Goal, team, Work, Attempt, Message, Schedule, artifact, Git and terminal-callback
primitives are sufficient. It does not assume a recurring workflow, `Mission` entity, heartbeat or
autonomous-improvement state machine.

## Hypothesis

> One standing non-producing Game Product lead can turn executable and playtest evidence into a series
> of bounded Staff-owned improvements, recover from local failure, and deliberately no-op at a scheduled
> review when the artifact has not produced a new reason to change.

The contrary result is that continuity loses the current product state, repeats work, treats elapsed
time as a reason to build, needs the owner to relay every result, or cannot recover a useful worker
checkpoint.

## Frozen run

Run one isolated `_test` company from the exact Dogfood 4 baseline. Use `zai/glm-5.3`, the same
persistent Runtime, no external effects and a USD 25 model ceiling. The founder's uncompleted taste
judgement is not silently converted into acceptance: the baseline is technically verified and remains
product-judgement pending.

Exec receives one mandate, appoints one accountable Game Product lead and returns to availability. The
lead remains a supervisor. Staff owns every source, test, evidence and commit change. The lead may add
an independent reviewer only when separate evidence is useful.

### Cycle 1 - playtest evidence

Deliver a structured observation against the existing native screenshots and runnable probe: the
networked loop is legible to executable checks, but the player-facing view is dominated by raw
telemetry, overlapping labels and weak moment-to-moment instruction. The lead must judge the smallest
improvement, commission Staff, inspect the resulting native target and either accept or revise it.

Redeliver the same signal with the same source key. It must not create duplicate production.

### Cycle 2 - executable regression

After Cycle 1 closes, inject one real parse/build regression into the isolated integration branch and
record the failing command. The failure, not evaluator prose, is the signal. The lead must preserve the
accepted Cycle 1 artifact, commission the smallest repair, require the positive end-to-end probe and the
previously incomplete negative delivery-zone probe, and promote only a clean exact Staff commit.

During one productive Attempt, terminate the exact worker process after useful filesystem or Git state
exists. Recovery must reuse the preserved state or exact checkpoint without owner repair, duplicate
Work or a new production actor racing the old one.

### Cycle 3 - scheduled review and quiet

The lead creates one durable, direct schedule for its own product review. When it becomes due, the
schedule must wake that lead rather than Exec. The wake receives the reason as a durable fact and
inspects current artifact, Work, test and playtest evidence. With no new decision-changing evidence,
the correct outcome is no new Work, no repository change and no reschedule merely to look active.

Observe a quiet interval after the turn. No model, Work, commit or artifact activity may appear.

## Success contract

1. The exact baseline is retained and freshly passes its positive host/client probe before mutation.
2. One standing lead supervises all cycles; Exec does not relay routine signals or join production.
3. Each accepted change is Staff-attributable through Work, Attempt, exact commit, gates and lead
   judgement.
4. Cycle 1 materially improves the native player-facing experience rather than only editing prose.
5. Duplicate playtest delivery produces no duplicate Work or artifact.
6. Cycle 2 begins from a genuinely failing executable probe and ends with positive and negative probes
   passing from a clean promoted commit.
7. Exact worker termination preserves useful work and recovers without owner implementation.
8. The direct schedule survives durable storage, wakes the accountable lead once and never wakes Exec.
9. Cycle 3 creates no production when no new evidence warrants it; the following quiet interval is
   exactly idle.
10. No external effect, public deployment, purchase, outreach or unlabelled simulated fact occurs.
11. Cost, elapsed time, model attempts, Work/Attempts, wake routing, owner interventions, failures and
    repository lineage are reported from source evidence.
12. The result chooses one disposition: `current primitives suffice`, `thin affordance`, `new concept
    earned`, `reject`, or `inconclusive`.

## Measures

- runnable outcome and product-quality delta per cycle;
- accepted Work, Attempts, revisions, commits and review loops;
- time from signal to accepted closure;
- model usage and estimated cost by actor;
- owner and Exec wakes by cause;
- duplicate signal to duplicate Work ratio;
- recovery time and useful state preserved after termination;
- scheduled-wake target, delivery count and restart behaviour;
- quiet-interval Work, model, event and repository deltas; and
- harness-caused versus game/product-caused failure.

## Failure and validity rules

- The founder's taste acceptance remains unknown until the founder uses the prepared native target.
- Model or reviewer agreement cannot prove fun. Executable checks prove mechanics; withheld-context
  criticism and founder review supply judgement.
- A schedule is a time fact and review opportunity, not permission to invent work or a completion
  signal.
- A worker's message is not terminal evidence. Runtime artifacts, gates and final Work state must land
  before the durable supervisor callback.
- Evaluator mutations are allowed only in this `_test` Runtime, are committed and labelled as injected,
  and never become product evidence themselves.
- Fixed time bounds observation only. Semantic terminal state decides completion.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| Cron manufactures perpetual feature churn | **Guarded** | One reasoned schedule, explicit no-op criterion and quiet interval |
| Model review is mistaken for founder taste | **Invariant** | Report it as agent evidence; preserve founder judgement as unknown |
| Failure injection contaminates the accepted artifact | **Guarded** | Isolated `_test` company, labelled commit and exact clean repair promotion |
| Schedule delivery is lost across a daemon crash | **Guarded** | Durable Schedule to durable Message handoff in one transaction plus live replay |
| Recurrence semantics grow into a workflow engine | **Accepted** | Recurrence is model judgement expressed by another one-shot schedule only when useful |
| More content is assumed to be better | **Guarded** | Every cycle needs observed evidence and a native outcome delta; no evidence permits no-op |

## Deliverables

1. Versioned Dogfood 4 v0.3 charter and isolated company configuration.
2. Frozen playtest and executable signal records.
3. Per-cycle Work, Attempt, artifact, Git, gate, wake and spend evidence.
4. Before/after native review targets and prepared founder review.
5. Recovery, duplicate and quiet-interval observations.
6. Concise result, machine-readable metrics, friction dispositions and architectural decision.
7. Updates to the evidence index and coordination canon only for supported claims.
