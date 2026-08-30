# S26-T5 — Separate feedback from interruption

New information is delivered at a safe checkpoint by default. Stopping productive work is an explicit,
authorised operation.

**Observed friction:** direct feedback arrived after an Attempt snapshot had frozen. The scheduler
automatically superseded the Attempt even when it was still producing the requested repair, multiplying
one small change into five or six attempts.

**Layer:** OrgIntel + Authority Plane.

**Deletion target:** automatic Attempt supersession on any new message and ambiguous cancellation facts.

## Scope

- Queue ordinary feedback against Work with causal source and delivery status.
- Add safe checkpoints at model-turn, tool-call and declared long-process boundaries.
- Add explicit urgent interrupt with actor/authority, reason and target Attempt.
- Preserve output from interrupted Attempts as unaccepted evidence; never merge it silently.
- Deliver a compact causal delta once, not the full message history repeatedly.

## Acceptance

- Ordinary feedback during a productive tool call is delivered at the next checkpoint and the Attempt
  keeps its identity.
- An authorised urgent interrupt stops the process group, records the reason and creates no replacement
  until the lead decides.
- Duplicate feedback is idempotent; feedback already present in frozen inputs is not redelivered.
- Wrong-lineage or dangerous-effect detection can use explicit system interruption and is audited.

