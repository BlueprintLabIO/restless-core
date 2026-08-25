# EXP-05 final results — demand, causal supervision and a wide company

**Completed:** 26 August 2026
**Disposition:** accepted scoped evidence; no new coordination primitive earned
**Primary model:** live `zai/glm-5.3` throughout
**External effects:** none; all work used fictional isolated `_test` companies

## Main answer

Teams are valuable when independent accepted work can overlap under real demand, not because a task
looks large. The 240-account all-at-once backlog crossed from Q1 to Q2 twice; the identical paced
population did not produce a p90 gain. Forty-entity monitoring also crossed cleanly at Q2 without
fan-in. Q4 sales was much faster but lost the frozen tail-quality comparison, making Q2 the current
accepted ceiling for the tested backlog.

Supervision is causally valuable during material change. An addressable non-producing lead redirected
two live support Attempts and closed 96/96 under the new policy; terminal-only delivery left 16 stale
cases and deadlocked. The causal lead still made a real judgement error, so supervision needs exact
evidence and outcome review rather than more status machinery.

Four independent leads and workers produced 90/90 exact company units while Exec had returned to
availability. That branch remains infrastructure-invalid for semantic comparison because one blind
evaluator omitted its decision twice. Product truth is preserved without pretending the measurement
passed.

## Sprint decisions

| Question | Result |
| --- | --- |
| G1 exact GLM/product path | passed: exact selector, effort, tools, resumable actor sessions and attributed usage |
| G2 sustained Q4 | passed: 32/32 exact units, four concurrent Staff model calls, $0.3876 |
| G3 event/cancellation truth | passed: exact Work interruption, preserved workspace, stale Attempt superseded in 0.382s |
| G4 local closure/evaluation | passed: order-independent exact indexes, ownership/corruption rejection and blind-schema falsification |
| Sales capacity | D0 Q2 replicated; D1/D2 did not cross; conditional Q4 stopped on tail quality |
| Support change | causal 96/96; terminal 80/96 counted outcome failure |
| Monitoring breadth | Q2 1.917× throughput, 0.554× p90 and 0.853× charged cost/unit; both accepted |
| Concurrent company | 90/90 product-exact; branch stopped evaluator-infrastructure-invalid |
| Fan-in / wildcard | none used or authorised |

Detailed results: [`sales`](t01-sales-demand-capacity.md), [`support`](t02-support-change.md),
[`monitoring`](t03-monitoring-breadth.md), [`company`](t04-company-concurrency.md),
[`team size`](demand-team-size-guide.md), [`supervisor span`](supervisor-span-guide.md), and
[`fan-in/wildcards`](fan-in-wildcard-dispositions.md).

## Harness defects repaired before closure

- exact non-Exec model and effort now reach capability signing, ACP launch, readiness, session and
  usage evidence rather than inheriting an Exec fallback;
- Docker company-image builds use a bounded context;
- Work feedback interrupts only the exact linked Work;
- provider metering drains independently of a cancelled ACP process so terminal cost is not lost;
- outcome failures and branch-stopped infrastructure failures are first-class terminal dispositions,
  never silently replayed or collapsed into generic invalid;
- expected concurrency follows the frozen arm roster;
- charged metered cost, not tokens or zero list-price fields, drives economic gates; and
- a malformed blind evaluation receives one retry, then stops the branch without inferred fields.

These repairs change evidence truth, not topology outcomes. The sales crossover decisions remain the
same after the charged-cost correction.

## Product implications

Retain the current minimal substrate: Actor, Work, Attempt, addressed messages, artifacts, schedules
and process callbacks. No queue engine, deterministic team router, shared history, blackboard or
workflow engine earned promotion.

Two narrow implementation problems remain:

1. exact outcome gates should deliver terminal failure evidence to the accountable lead, not expose
   success-only callbacks that can leave a semantically finished outcome waiting forever;
2. Exec's fourth dispatch took 68.68 seconds synchronously even though the organisational boundary was
   correct; durable acceptance should return portfolio availability before lead orientation finishes.

The evaluator's missing `decision` is an experiment-harness schema problem and should be constrained
there rather than becoming product machinery.

## Experiment-only adapter disposition

The fixture builder, fictional corpora, arm catalogue, product runner, exact validators, blind rubric,
analyzer, completion audit and bounded result directories remain quarantined under `EXP-05` solely for
reproducibility. They are not imported into OrgIntel or Runtime as a queue system, evaluator service,
team policy or second lifecycle. The product-path bug fixes above remain in their owning code because
live evidence required them. No losing topology or wildcard adapter survives in production.

## Limits and next frontier

All semantic evaluations used the same model family as production, so evaluator correlation remains.
The queues are fictional, no external input or effect occurred, and company-level lead-message counts
were not observable. Q2/Q4 numbers are contingent on this provider, model, effort and fixture.

The next useful work is an implementation consolidation, then real inbound-signal dogfood in an
isolated company: receive a provider-observed event, durably dispatch it, deliver exact success or
failure evidence to the responsible lead, prepare a native outcome and resume without polling. A new
coordination experiment should wait until that real path exposes a repeated bottleneck.
