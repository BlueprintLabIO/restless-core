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

## Consequences

- Owner requests receive fast executive triage and durable lead ownership.
- Several departments or outcomes can continue concurrently without occupying Exec.
- The lead, not Exec, is the strong singleton baseline in delegation crossover experiments.
- Work between Exec and lead records the outcome boundary; Work below the lead remains sparse and
  exists only for real cross-actor commitments.
- Material callbacks wake Exec only when they require company-level judgement; ordinary integration
  and recovery remain with the lead.
- Exec needs concise portfolio state and prepared native evidence, not a production checkout or the
  full implementation transcript.
