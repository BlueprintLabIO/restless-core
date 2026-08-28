# Coordination theory

**Status:** Working explanatory theory
**Date:** 26 August 2026
**Authority:** Subordinate to `ARCHITECTURE.md`; empirical claims are indexed in
`experiment/coordination/EVIDENCE.md`.

## Central proposition

> **An organisation is bounded minds exchanging lossy information to produce coherent outcomes.**

Every additional mind can add capacity, parallelism, specialisation, independent evidence or
alignment and recovery attention. Every boundary also adds briefing, context loss, communication,
integration, review and failure amplification. A team is useful only when the first set exceeds the
second:

```text
net value of another mind
= parallelism + specialisation + independent evidence + added capacity + alignment/early repair
- briefing - context loss - communication - integration - review - amplified error
```

This is an explanatory inequality, not a runtime score. Its terms depend on the work, actors, tools
and moment; a model should judge them from the actual situation.

## The abstraction test

Place a boundary according to **where accountability closes**, not task size, file count or estimated
duration.

| Level | Owns | Completion boundary |
| --- | --- | --- |
| **Exec** | Portfolio, company priorities, resource conflicts and authority escalation | The right outcomes have accountable owners and company-level exceptions are decided |
| **Accountable lead** | One coherent business, product or operational outcome | The complete native result can be inspected, defended and presented |
| **Staff** | One independently useful semantic responsibility or locally closing unit inside a lead's outcome | Its artifact, observation, decision or attributed unit result is ready for lead-level integration or aggregation |
| **Hand/tool** | A bounded operation chosen by the lead | The requested measurement, build, search, render or transform returns; it owns no project judgement |

Derived rules:

1. Create a separate lead when an outcome can progress and be judged independently, has its own
   continuing trade-offs, and can be held accountable without another lead completing its meaning.
2. Use Staff when another mind can own a stable, independently useful contribution or a distinct unit
   in a repeated queue, while the parent lead still owns whole-outcome completion.
3. Use a hand or ordinary tool when the lead can specify the operation and interpret its return
   without transferring semantic ownership. A hidden cognitive coauthor is Staff, not a hand.
4. Give one end-to-end worker the whole production boundary when state is volatile, judgement is
   tightly coupled or another producer would mostly require transferring context.
5. If two nominal leads must continuously share state or jointly declare one result complete, the
   boundary is probably wrong: place the outcome under one accountable lead.

## Coordination between leads

Leads are independent by default, not isolated. They coordinate at interfaces:

```text
direct material fact, decision or artifact → affected lead
hard deliverable dependency              → sparse Work edge
priority, resource, charter or strategy conflict → Exec
```

One lead remains the integrator for any shared outcome. Direct peer communication carries only
changed information that can alter another lead's work. Exec is the portfolio arbitrator, not a
status relay or common-room moderator.

Hierarchy is therefore communication compression: it replaces a many-to-many conversation with local
autonomy, one accountable interface per outcome and exception escalation. It is valuable only while
it reduces total coordination; hierarchy that merely forwards messages is overhead.

## Decomposition and replication are different

Most coordination experiments ask several minds to produce **one shared result**. Sales, support,
recruiting, moderation and case processing often present a different problem: **many independently
valuable units of the same work**. A hundred prospects do not need a hundred-way design discussion.
They need attributable owners, a shared charter and enough parallel capacity to act while each unit is
valuable.

In replicated work, similarity is not necessarily duplication. Two capable actors using the same
playbook on different accounts can add nearly independent value. Briefing and calibration can be
amortised across many units, while integration becomes aggregate measurement, sampling and exception
handling rather than a lead rereading and merging every artifact. Team size should then follow viable
backlog, time sensitivity, external capacity, quality distribution and marginal unit economics—not
the semantic size of one task.

Local closure removes one avoidable serial boundary; it does not remove fixed actor orientation,
provider admission, tool cadence or supervisor judgement. Parallel capacity appears only when the
value of backlog reduction or response time is large enough to amortise those costs. A small frozen
queue can therefore be perfectly partitioned and still favour one worker. This is the central
inequality applied to replication rather than coauthoring.

EXP-05 sharpens “backlog” into a temporal claim. The same 240-unit account population crossed from
Q1 to Q2 twice when all units were waiting, yet produced no p90 improvement when arrivals were paced.
Forty-entity monitoring did cross at Q2 because disjoint search breadth could overlap immediately.
Total population and department label are therefore weak signals; the lead should judge whether
valuable independent work is simultaneously waiting while current Staff is occupied.

The Work graph should remain sparse. It records an actor's real queue, batch, territory or campaign
responsibility and observable results; it need not mirror every call, prospect or support ticket as a
new organisational plan node.

## Supervision is a distinct required function

Restless makes lead/worker separation an owner-decided product invariant. The accountable lead retains
the broad mission context and independent judgement while workers enter narrow production contexts. It
frames, commissions, observes, guides, redirects, repairs through workers and accepts or rejects the
exact outcome. It does no planned production, silent artifact repair or private parallel implementation.

This separation can catch drift, absorb Exec updates and prevent worker confidence from becoming the
company's final judgement. It also spends another mind, adds a handoff and can increase local latency.
EXP-03 measures that premium rather than treating a cheap direct actor as permission to delete the
function. Supervisor availability is event-driven through material updates and artifacts, not
maintained by polling or status meetings. Event-driven does not mean every event demands an immediate
model wake: ordinary successful partition completions may accumulate durably until a local batch
closes. Harm, blockage, changed policy, contradiction or another decision-relevant exception remains
urgent.

Availability is not correctness. In EXP-05, causal policy delivery let the lead supersede two stale
Attempts and close 96/96 cases under the new policy, while terminal-only delivery left 16 stale cases
and deadlocked. The causal lead still introduced a substantive unit-level judgement defect. The
supervisor therefore needs exact changed facts, addressed Work and independent outcome evidence; a
better wake path does not justify trusting the wake's judgement without review.

EXP-09 adds the completion boundary. A worker's progress note is not terminal evidence and can reach
the lead before Runtime gates finish. The Runtime therefore observes artifacts, gates and final Work
state first, then delivers one durable Work-linked fact through a recoverable Attempt outbox. Staff
uses direct mail only for a new fact or contradiction that needs judgement before completion. This is
event-driven supervision: immediate on the material fact, recoverable after restart, and independent
of timeouts or heartbeats.

Continuous responsibility does not imply continuous execution. A standing lead may retain a mandate
across many signals while every useful production cycle remains bounded Work and the correct response
to irrelevant, duplicate, stale or absent evidence is quiet. EXP-09's direct standing editorial lead
used materially less owner, Exec and model activity than relay while matching useful behavior; its
opportunity lead completed useful updates and produced zero quiet-interval activity. No new Mission
entity or workflow engine follows from that result.

EXP-10 extends that result to a playable product and a time-driven review. One standing lead closed two
useful Staff-owned game cycles, recovered a killed productive process, suppressed an exact duplicate
and then correctly did nothing at a scheduled inspection. The schedule was a one-shot durable fact
addressed to that lead. It did not recur, wake Exec or manufacture a reason to build. Continuous
company operation therefore combines material external or internal events with occasional reasoned
time facts; neither is a heartbeat, and both may end in quiet.

The same run shows where mechanical support still matters. Deterministic acceptance gates are
operational evidence, not immutable doctrine: a mistaken command must be retired with its history
preserved and replaced without inventing a new outcome. Missing final usage after observable text or
tool activity is likewise incomplete evidence, not proof that the model never ran. These are recovery
facts below the lead's judgement, not reasons to script how the lead plans or communicates.

## Why fast agents change the shape

Fast inference makes an individual worker's production cheap relative to briefing and integration. A
worker may finish a task before an additional mind can orient, so the current crossover sits farther
toward one producer than in many human organisations. Current LLMs also raise the boundary cost through
weak collaborator-state modelling, lossy handoffs, correlated mistakes and inconsistent integration.

This predicts a **wide, shallow and elastic** organisation:

```text
continuously available Exec
├── supervisor lead + one end-to-end worker
├── supervisor lead + one end-to-end worker
├── supervisor lead that temporarily adds Staff when saturated
└── accountable lead with an elastic Staff pool for repeated independent units
```

It does not eliminate teams. Teams remain valuable where work exceeds one effective context/session,
contains genuinely independent evidence or expertise, permits useful latency overlap, or needs
separate observation to reduce consequential uncertainty. Better models, persistent context or
cheaper coordination may move the crossover again. Replicated throughput is a separate crossover:
even a very fast lead remains serial while a valuable queue can grow in parallel.

## Enduring conclusions

- Use the smallest effective number of minds.
- Distinguish decomposing one shared outcome from replicating independently valuable units.
- Preserve the lead as a non-producing mission keeper and independent final judge.
- Give every outcome one accountable integrator.
- Split only across stable, independently valuable seams.
- Communicate changed facts, decisions and artifacts rather than activity.
- Let observable reality arbitrate checkable questions; agreement is not evidence.
- Keep local work autonomous and escalate only genuine cross-boundary exceptions.
- Change the organisation as coupling, uncertainty and saturation change.
- Treat local closure as a prerequisite for queue scaling, not proof that another worker will win.
- Preserve actor identity through isolated resumable sessions and factual state; processes may be replaced.

## Contingent conclusions

These are current priors, not permanent truths:

- one end-to-end worker is the default below that worker's effective saturation point for a coherent
  shared outcome, not necessarily for a queue of independent units;
- an additional autonomous Staff member usually loses unless it adds differentiated outcome value;
- same-evidence criticism is usually an echo rather than independent review;
- current model assignments, team-size guidance, ACP, Work schemas, token speeds and provider limits;
- current effort-tier behaviour, cache/usage visibility and provider concurrency envelope;
- any fixed numerical routing threshold.

The exact Work graph is not the theory. The enduring requirement is a small factual substrate that can
show who really accepted responsibility, what evidence returned and who judged the combined outcome.

## Experimental consequences

EXP-05 completed the next queue test. Q2 crossed twice under an all-at-once 240-account backlog and
once on 40-entity monitoring breadth; paced sales arrivals did not improve p90. Q4 sales had real
four-way provider capacity and much higher throughput but lost marginal tail quality. This supports a
wide, shallow, elastic organisation in scoped demand regions, not a fixed team-size table.

The four boundaries remain useful as a reasoning map:

```text
shared outcome:   one worker ↔ parallel bounded hands ↔ additional autonomous Staff
portfolio:        one broad supervised outcome ↔ separate accountable leads
replicated units: one unit worker ↔ elastic same-role Staff pool with local closure
effort:           validated model/workload effort ↔ consequence escalation + dynamic closure headroom
```

Future experiments must still compare accepted native outcomes, wall time, newly processed input,
tool/runtime cost, integration work and owner/Exec attention. Replicated work additionally measures
accepted units per time, marginal unit economics, tail quality, backlog and lead bottlenecks without a
model assembler or per-unit management wake. Effort remains configured and observable without private
chain of thought. A topology wins only in the work region it improves.

The next useful frontier is implementation truth rather than another wildcard search:

1. make Exec dispatch durably accepted and asynchronously return portfolio availability before lead
   model orientation completes;
2. constrain experiment evaluator structure without inferring missing semantic decisions or hidden
   representation requirements;
3. project one provider-observed inbound signal to the nearest accountable lead without owner relay;
4. distinguish a package prepared for future owner judgement from owner attention genuinely owed now;
   and
5. dogfood the standing-responsibility pattern through governed native review and real effects.

Reopen a coordination mechanism only if that real path repeats one causal information-flow failure.
No shared history, blackboard, fan-in, Mission entity or workflow engine is implied by EXP-09.
