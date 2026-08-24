# Coordination theory

**Status:** Working explanatory theory
**Date:** 24 August 2026
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
| **Lead directly** | Coupled reasoning and execution that benefits from one causal model | No cross-actor responsibility boundary is useful |

Derived rules:

1. Create a separate lead when an outcome can progress and be judged independently, has its own
   continuing trade-offs, and can be held accountable without another lead completing its meaning.
2. Use Staff when another mind can own a stable, independently useful contribution or a distinct unit
   in a repeated queue, while the parent lead still owns whole-outcome completion.
3. Use a hand or ordinary tool when the lead can specify the operation and interpret its return
   without transferring semantic ownership. A hidden cognitive coauthor is Staff, not a hand.
4. Keep work inside the lead when state is volatile, judgement is tightly coupled or the contribution
   would mostly require transferring the lead's context.
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

The Work graph should remain sparse. It records an actor's real queue, batch, territory or campaign
responsibility and observable results; it need not mirror every call, prospect or support ticket as a
new organisational plan node.

## Leadership posture is a separate variable

Accountability does not decide whether a lead should produce. A lead can be a **player-coach** that
does complementary work, or a **supervisor** that preserves attention for mission alignment,
observation, guidance, redirection, recovery and final judgement.

A separate supervisor retains broader mission context while a worker carries narrow production
context. That separation can catch drift, absorb Exec updates and repair churn before it compounds.
It also spends another mind on oversight, adds a handoff and may merely echo a capable worker. With
one short, stable task and one strong worker, full-time supervision may cost more than it saves. As
worker count, duration, volatility, capability uncertainty, consequence or repair cost rises,
supervision has more opportunity to pay.

The posture should therefore be elastic. A lead may begin as player-coach, reserve more attention as
coordination load rises, and take over production only after an observable exception. Supervisor
availability should be event-driven through material updates and artifacts, not maintained by polling
or status meetings.

## Why fast agents change the shape

Fast inference makes an individual lead's production cheap relative to briefing and integration. A
lead may finish a task before a second mind can orient, so the current crossover sits farther toward
solo work than in many human organisations. Current LLMs also raise the boundary cost through weak
collaborator-state modelling, lossy handoffs, correlated mistakes and inconsistent integration.

This predicts a **wide, shallow and elastic** organisation:

```text
continuously available Exec
├── mostly-solo accountable lead
├── mostly-solo accountable lead
├── accountable lead that temporarily adds Staff when saturated
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
- Treat player-coach versus supervisor-only as a lead posture, not a permanent role doctrine.
- Give every outcome one accountable integrator.
- Split only across stable, independently valuable seams.
- Communicate changed facts, decisions and artifacts rather than activity.
- Let observable reality arbitrate checkable questions; agreement is not evidence.
- Keep local work autonomous and escalate only genuine cross-boundary exceptions.
- Change the organisation as coupling, uncertainty and saturation change.

## Contingent conclusions

These are current priors, not permanent truths:

- zero Staff is the default below one strong lead's effective saturation point for a coherent shared
  outcome, not necessarily for a queue of independent units;
- one autonomous Staff member usually loses unless it adds differentiated outcome value;
- same-evidence criticism is usually an echo rather than independent review;
- current model assignments, team-size guidance, ACP, Work schemas, token speeds and provider limits;
- any fixed numerical routing threshold.

The exact Work graph is not the theory. The enduring requirement is a small factual substrate that can
show who really accepted responsibility, what evidence returned and who judged the combined outcome.

## Experimental consequences

The next useful frontier is not another broad search for teamwork tricks. It is to locate three
related boundaries:

```text
shared outcome:   lead direct ↔ parallel bounded hands ↔ autonomous Staff
portfolio:        one broad lead ↔ separate accountable leads
replicated units: one producer ↔ elastic same-role Staff pool
lead posture:     player-coach ↔ supervisor-only
```

The experiment must compare accepted native outcomes, wall time, newly processed input, tool/runtime
cost, integration work and owner/Exec attention. Replicated work must additionally measure accepted
units per time, marginal unit economics, tail quality, backlog and lead bottlenecks. A topology wins
only in the work region it improves; no result becomes a universal router.
