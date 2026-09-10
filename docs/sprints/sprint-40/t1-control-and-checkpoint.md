# S40-T1 — Add one recoverable controller and checkpoint

**Layer:** OrgIntel + Runtime Bridge  
**Serves:** Human and autonomous control must not race or lose the pre-intervention workspace.

## Work

- Add the smallest controller state attached to the existing Session/Attempt and Work relationship.
- Implement requested, transferring, owner-controlled, uncertain, returning and agent-controlled
  transitions with one idempotent owner identity and bounded expiry.
- Reach a declared safe turn boundary before transfer and refuse replacement/autonomous prompts while
  owner control is active or uncertain.
- Capture exact Git/worktree, dirty state, process/session identity and a recoverable checkpoint before
  issuing native access.
- Reconcile restart, duplicate request, abandoned transfer and actor/Work reassignment.

## Acceptance

Concurrency tests prove at most one controller and no new autonomous writer during owner control.
Checkpoint restoration returns the exact pre-transfer state without putting ordinary file custody or a
universal workspace lease into Authority.

## Makes deletable

Process-local “paused” flags, inferred controller state and manual pre-takeover Git commands.
