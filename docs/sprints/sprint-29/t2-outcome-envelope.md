# S29-T2 — Preserve an outcome's effective standard and limits

**Layer:** Owner input + OrgIntel team/Work context.

**Serves:** Sprint 29 criteria 1, 2, 8, 10 and 14.

**Depends on:** S29-T0 and the shared type from S29-T1.

**Observed friction:** An owner's explicit ambition can be present at send time yet disappear when the
Exec commissions a team, creates Work, hands off or receives revision feedback.

**Makes deletable:** Prose parsing, duplicated team-specific quality flags and reconstructed defaults
after the outcome has begun.

## Outcome

The ordinary owner request carries an optional explicit override. At commission, Restless resolves
the explicit selection, a clearly expressed natural-language instruction or the company default once,
records the effective standard and its source in the existing team charter/Work context, and
preserves it through continuation, feedback, handoff and restart.

## Scope

- Extend the owner message/directive input narrowly so an explicit selection is durable and
  attributable; do not use generic metadata JSON or parse message text.
- Freeze and test the inheritance order in the sprint spec.
- Let the accountable Exec infer a natural-language override only when the instruction is clear;
  preserve the source message and rationale rather than adding keyword or regular-expression rules.
- Record `company_default`, `owner_override` or `owner_language` as the selection source at commission.
- Preserve the effective value for revisions and continuations of the same commissioned outcome.
- Represent an optional target deadline and ask-before-crossing spend envelope only where units,
  ownership and accounting semantics are explicit.
- Keep the hard company ceiling source-owned and visibly distinct from any outcome envelope.
- Expose exact absence/unknown attribution rather than synthesizing zero spend or unlimited time.

## Acceptance

- Tests cover default inheritance, explicit override, continuation, revision, handoff, restart and a
  later company-default change.
- Changing the company default does not rewrite already commissioned outcomes.
- An ordinary message with no new commission does not accidentally create a second standard scope.
- Clear natural-language instructions are interpreted by the accountable Exec and shown back;
  ambiguous language inherits the default, and deterministic matching cannot mutate policy.
- The Exec and lead receive one canonical effective value and selection source.
- Advisory and hard limits cannot be confused in serialized or owner-visible contracts.
- No project entity, generic policy envelope or parallel Work lifecycle is introduced.

## Branch and purge

Observe the current owner-message, team-charter and Work seams before choosing the smallest durable
shape. Compare a narrow linked request record with extending an existing source record; do not retain
both. A generic message metadata bag is not an acceptable losing branch.
