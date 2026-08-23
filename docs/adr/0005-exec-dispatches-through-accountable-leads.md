# ADR 0005 — Exec dispatches through accountable team leads

**Status:** Accepted

## Decision

The singleton Exec is the continuously available company-level dispatcher. Every owner request that
requires execution is delegated to exactly one accountable team lead. The lead may be a standing
department lead or a temporary outcome lead. If no suitable lead exists, Exec appoints one before
productive work begins.

The team lead owns the complete outcome: decomposition, direct work, optional Staff delegation,
canonical integration, review preparation and completion judgement. A lead may work alone when the
outcome is tightly coupled; “team lead” is an accountability and concurrency boundary, not a required
team size.

Before choosing Staff, the lead builds a causal understanding of the outcome and chooses the smallest
effective team, including itself alone. If another actor can own a stable seam, the lead communicates
purpose, current understanding, unknowns and observable proof naturally, continues complementary work,
then personally inspects and integrates the returned artifact. OrgIntel Work proves that the
cross-actor contribution happened; it does not prescribe the lead's plan or conversation.

Exec retains owner-mandate interpretation, portfolio prioritisation, resource allocation,
cross-department arbitration, authority escalation and company-level continue/pivot/stop decisions.
After dispatch it ends the wake rather than waiting, polling, producing or integrating, leaving it
available for the next owner request while departments run concurrently.

## Why

A company naturally has multiple departments advancing in parallel. Making Exec the producer or
project integrator serialises those departments through the one actor that must remain responsive to
the owner and the whole portfolio.

The v23 coordination experiment showed that one strong actor beat a lead-plus-worker arrangement on a
tightly coupled game slice. The correct organisational interpretation is not “Exec should do the
work.” It is “the accountable team lead should be allowed to do tightly coupled work alone.” The
experiment conflated company executive and project lead; this decision separates them.

The EXP-01 natural-lead screen then compared a forced handoff, fresh solo lead and an optional one-Staff
lead on the same accepted G-WORLD outcome. Natural leadership won blind quality 8.2 versus 6.4 and 4.5,
while remaining slower than solo. Its first run also narrated a Staff contribution that did not exist;
the empty Work/Attempt trace exposed the false account, and one factual `commission` clarification
produced genuine complementary work without a coordination workflow. This supports natural judgement
above a sparse factual substrate, not a universal claim that teams are faster or always better.

## Consequences

- Owner requests receive fast executive triage and durable lead ownership.
- Several departments or outcomes can continue concurrently without occupying Exec.
- The lead, not Exec, is the strong singleton baseline in delegation crossover experiments.
- Work between Exec and lead records the outcome boundary; Work below the lead remains sparse and
  exists only for real cross-actor commitments.
- The same lead identity and natural-team contract apply on productive Work and conversation wakes;
  the runtime must not demote a lead to generic specialist on one path.
- No teamwork-pattern recommender, handoff form, message cadence or graph-shaped execution plan is part
  of this decision.
- Material callbacks wake Exec only when they require company-level judgement; ordinary integration
  and recovery remain with the lead.
- Exec needs concise portfolio state and prepared native evidence, not a production checkout or the
  full implementation transcript.
