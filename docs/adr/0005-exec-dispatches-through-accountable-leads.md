# ADR 0005 — Exec dispatches through accountable team leads

**Status:** Accepted

## Decision

The singleton Exec is the continuously available company-level dispatcher. Every owner request that
requires execution is delegated to exactly one accountable team lead. The lead may be a standing
department lead or a temporary outcome lead. If no suitable lead exists, Exec appoints one before
productive work begins.

The team lead owns the complete outcome: decomposition, worker selection and commissioning,
supervision, exact artifact promotion, review preparation and completion judgement. The lead performs
no planned production, silent artifact repair or private parallel implementation. Every executable
outcome therefore has at least one Staff worker; “team lead” is an accountability and independent-
judgement boundary, not the artifact producer.

The non-producing posture is an owner decision. It remains valuable as a distinct mission-preserving,
guidance, redirection, recovery and final-judgement function even when a direct actor would finish a
bounded task faster.

The lead builds a causal understanding of the outcome and chooses the smallest effective worker set,
starting with one end-to-end worker. If another actor can own a stable seam, the lead communicates
purpose, current understanding, unknowns and observable proof naturally, then personally inspects and
promotes the exact returned artifact or sends correction back to a worker. OrgIntel Work proves that
the cross-actor contribution happened; it does not prescribe the lead's plan or conversation.

The organisational level is determined by accountability closure rather than apparent task size. A
separate lead owns an independently judgeable outcome; Staff owns an independently useful semantic
contribution or locally closing repeated unit inside that outcome; a hand or ordinary tool performs a
bounded operation without owning project judgement. Several leads communicate material facts and
artifacts directly, while only portfolio, resource, charter and strategy conflicts rise to Exec. The
fuller explanatory model is in [`docs/COORDINATION_THEORY.md`](../COORDINATION_THEORY.md).

Exec retains owner-mandate interpretation, portfolio prioritisation, resource allocation,
cross-department arbitration, authority escalation and company-level continue/pivot/stop decisions.
After dispatch it ends the wake rather than waiting, polling, producing or integrating, leaving it
available for the next owner request while departments run concurrently.

## Why

A company naturally has multiple departments advancing in parallel. Making Exec the producer or
project integrator serialises those departments through the one actor that must remain responsive to
the owner and the whole portfolio.

The v23 coordination experiment showed that one strong actor beat a lead-plus-worker arrangement on a
tightly coupled game slice. That result measures the local supervisory premium; it does not erase the
owner-decided need for a separate mission keeper and final judge. The experiment conflated company
executive, accountable supervision and production; this decision separates all three.

The EXP-01 natural-lead screen then compared a forced handoff, fresh solo lead and an optional one-Staff
lead on the same accepted G-WORLD outcome. Natural leadership won blind quality 8.2 versus 6.4 and 4.5,
while remaining slower than solo. Its first run also narrated a Staff contribution that did not exist;
the empty Work/Attempt trace exposed the false account, and one factual `commission` clarification
produced genuine complementary work without a coordination workflow. This supports natural judgement
above a sparse factual substrate, not a universal claim that teams are faster or always better.

## Consequences

- Owner requests receive fast executive triage and durable lead ownership.
- Several departments or outcomes can continue concurrently without occupying Exec.
- One end-to-end worker under a non-producing lead is the minimum canonical baseline in delegation
  crossover experiments; direct actors remain diagnostic only.
- The team charter and Exec→lead message record the outcome boundary. Work is production responsibility
  and therefore begins below the lead, remaining sparse and factual for real Staff commitments.
- The current likely organisation is wide and shallow: several supervisor leads run concurrently with
  one worker by default and add Staff elastically. Repeated independent-unit departments may support
  large pools only where units close locally without model assembly. This is an evidence-scoped prior,
  not a fixed team size or span of control.
- The same lead identity and natural-team contract apply on every coordination and review wake; the
  runtime must refuse any path that schedules the lead as a productive Work owner.
- No teamwork-pattern recommender, handoff form, message cadence or graph-shaped execution plan is part
  of this decision.
- Material callbacks wake Exec only when they require company-level judgement; ordinary integration
  and recovery remain with the lead.
- Exec needs concise portfolio state and prepared native evidence, not a production checkout or the
  full implementation transcript.
