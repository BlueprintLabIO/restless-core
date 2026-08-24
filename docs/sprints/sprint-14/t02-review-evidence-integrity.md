# S14-T2 — Preserve completed source while preparing review evidence

**Layer:** Company Runtime plus OrgIntel outcome review.

**Observed friction served:** A completed Cosmon Attempt had a clean terminal observation, then a later
factual coordination wake regenerated tracked screenshots in that source worktree. That makes the
review surface harder to trust even when the linked candidate commit remains sound.

## Outcome

A post-completion review needing executable inspection receives a prepared ordinary review copy or
explicit evidence-output location. It may produce review files there, but cannot silently reuse the
completed Attempt’s source worktree as scratch space.

## Acceptance

- The source Attempt’s recorded commit and worktree coordinates remain the authoritative candidate.
- A prepared review location is tied to the same Attempt through existing artifact/reference fields,
  not a new custody lifecycle or persistent review state machine.
- If the exact source cannot be prepared safely, the system presents an honest unavailable review
  target rather than writing into the completed source checkout.
- A focused Runtime/OrgIntel scenario proves review preparation leaves the candidate worktree’s Git
  status and recorded commit unchanged.
- Review-only output is clearly marked supporting evidence, never an unlinked replacement candidate.
- Existing native ReviewTarget behaviour remains usable for a live site, rendered document or
  executable product scenario.

## Non-goals

- a per-turn disposable sandbox;
- cloning/importing assets into a custody system;
- a universal renderer or browser-automation platform;
- automatic judgement that a review is accepted.

## Deletion target

Uncontrolled post-completion tool output in the source Attempt worktree.

## Evidence

- `staff::workspace::tests::detached_review_copy_keeps_completed_source_commit_and_status_unchanged`
  passed against a real local Git repository: a review-only file in the detached copy left the
  completed source commit and status unchanged.
- Owner-work-linked Staff review preparation adds an attempt-bound `review_copy` reference labelled
  supporting evidence, never a replacement candidate; the live-Postgres smoke scenario passed that
  linkage.
- ACP execution now receives the requested review workdir as both its Docker working directory and
  ACP session location. The focused ACP workdir test passed.
