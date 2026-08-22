# Current coordination canon

Last updated: 23 August 2026

This is the one active synthesis. It stays small. Evidence details belong in `EVIDENCE.md` and the
linked run reports.

| ID | State | Current belief | Scope and reason it may change |
|---|---|---|---|
| **CL-001** | Accepted decision | Exec dispatches every executable owner request to one accountable standing or temporary team lead, then returns to availability. | Company operating model. Revisit only if real concurrent departments cannot be served by this boundary. |
| **CL-002** | Provisional | For tightly coupled production, the accountable lead should be allowed to work alone rather than being required to add producers. | One matched Cosmon pair. Requires replication and non-coding tests. |
| **CL-003** | Provisional | Decomposability, shared-state coupling and evidence boundaries predict useful Staff better than estimated task size alone. | Supported by v23 and current research priors; the local crossover curve is not measured. |
| **CL-004** | Provisional | Independent artifact criticism separates more cleanly and creates more information than parallel coauthoring on tightly coupled work. | Strong v23 critic artifact, but its callback failed and the finding has not been replicated. |
| **CL-005** | Accepted decision | Work is a sparse record of real cross-actor responsibility, not the lead's default plan or thought graph. | Architecture decision; relax only if repeated recovery failures require more durable ownership state. |
| **CL-006** | Provisional | Material events and artifact observation should drive resumption; elapsed time is an operating envelope, not semantic completion. | Multiple lab failures from polling/timeouts; production proof remains incomplete. |
| **CL-007** | Provisional | Observable artifacts should survive missing callback ceremony and wake the accountable lead as `unknown`, never as inferred success. | Deterministic fault probes plus v23 critic failure. Production implementation remains open. |
| **CL-008** | Hypothesis | A shared conversational prefix can reduce briefing and reintegration cost enough to move the delegation crossover frontier. | First wildcard to test; correlated errors and authority leakage may offset the gain. |

## Current routing prior

```text
Owner request
→ available Exec chooses one accountable lead and an envelope
→ lead owns the complete outcome
→ lead works alone when coupling is high
→ lead adds Staff for separable evidence/artifact branches
→ independent critic when hidden error or subjective quality warrants it
→ material exception returns to Exec only at portfolio altitude
```

This is a prior, not a hard-coded router. The wildcard programme tests whether new context and
communication structures make collaboration worthwhile in cases where ordinary handoffs currently
lose.

## Unknowns that matter now

- How much of team overhead is initial briefing versus ongoing mutual-state maintenance?
- Can shared-history forks retain a common causal model without creating correlated blind spots?
- Is one cognitive lead with parallel “hands” superior to several autonomous minds for coupled work?
- Which artifact and event boundaries generalise from code to research, documents and live operations?
- Can a cheap observable signal decide when communication is worth opening?
- Does any winning mechanism repay its implementation and operating complexity in production Restless?
