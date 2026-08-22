# S11-T2 · Commission repository Work with atomic ordered acceptance

**Layer:** OrgIntel owns Work/Attempt/dependencies/gates; Runtime executes the claimed Attempt and
direct gate processes.

**Observed friction:** Exec performed application repair itself, a schedulable Work node existed
before its gates, acceptance lived only in prose and piped shell output masked failing tests. Gates
sharing a timestamp later ran in UUID order despite mutating the same Attempt workspace.

## Outcome

Exec commissions substantial production through repository-bound Staff Work. Initial Work, graph
edges and ordered argv gates commit together. Every Attempt and repair runs those gates directly in
its current workspace; model judgement proposes completion but cannot settle a failed check.

## Acceptance

- A transaction failure leaves no Work, edge or gate visible to the scheduler.
- Repository name/base ref and every exact exit-code requirement are present at creation.
- Gate order equals declaration order, including after migration of existing Work.
- A generated-state fixture proves step two observes step one's output.
- A deliberately misleading `outcome_met` with one non-zero gate becomes a failed/blocked Attempt
  and wakes the coordinator with the exact gate evidence.
- Exec frames and reviews the work but does not edit the application source.

## Deletion

Makes late initial-gate attachment, timestamp/UUID gate ordering, prose-only release gates and
model-reported deterministic success deletable.
