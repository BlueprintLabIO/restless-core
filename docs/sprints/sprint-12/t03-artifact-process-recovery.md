# S12-T3 — Reconcile process and artifact evidence

**Layer:** Company Runtime + OrgIntel.

**Observed friction served:** structured completion is evidence-gated, but a cognitive process crash
or supervisor restart currently marks the Attempt failed and often sends a generic note to Exec. A
useful linked artifact or changed Git worktree can survive without a semantic callback and remain
organisationally stranded.

## Outcome

Every Staff Attempt records bounded process/workspace observations. When semantic completion is
missing, the execution mechanism fails closed while productive outcome stays explicitly unknown.
Existing artifact references and changed Git state are preserved in one recovery capsule addressed to
the accountable lead. Reconciliation never launches a duplicate Attempt.

## Acceptance

- Attempt launch records the exact worktree and starting Git observation when a repository is bound.
- Normal terminal, transport/process error and supervisor-orphan paths record one terminal process
  observation.
- A process error does not call the artifact good or bad; Work blocks with explicit unknown-outcome
  wording.
- Existing Attempt artifact references appear in the recovery capsule.
- A changed commit or dirty worktree becomes a bounded Runtime observation linked to the same Attempt,
  never a completion decision or copied custody artifact.
- The capsule names Work, Attempt, actor, workspace, start/end Git facts, linked artifacts and the
  smallest next judgement: inspect, revise, resume, reassign or abandon.
- The lead—not Exec—receives member recovery unless no accountable lead exists.
- Repeated reconciliation is idempotent and cannot create duplicate recovery references, messages or
  Attempts.
- `_test` controls cover crash after commit, crash after linked file, crash with no change and daemon
  restart while the process is live.

## Non-goals

- inferring semantic success from files, exit code or elapsed time;
- a new Work lifecycle, process database or artifact custody protocol;
- automatically rerunning the model.

## Deletion target

Generic “failed; workspace preserved” notices, callback-only discovery and restart-by-default repair.
