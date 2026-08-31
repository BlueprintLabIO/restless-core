# S28-T1 — Preserve meaning in narrow source-backed contracts

**Layer:** OrgIntel + Authority Plane + owner projection.

**Serves:** Sprint 28 criteria 3, 4, 5, 6, 10 and 12.

**Depends on:** S28-T0.

**Observed friction:** The source often knows the real action and state, while prose and Markdown carry
the distinction between trigger, effect, recommendation, choice and consequence in the owner view.

**Makes deletable:** Renderer parsing of prose, UI-invented action meaning and duplicated causal copy.

## Outcome

The owner projection carries enough typed source truth and attributed authored meaning for each
supported surface to render honestly without interpreting prose. It remains a projection over
existing Authority, owner handoff, Work, message and artifact concepts.

## Scope

- Compare the frozen corpus with the existing `OwnerBrief`, `AttentionItem`, `AttentionAction`, Work,
  artifact-reference, message-intent and decision-continuation contracts.
- Tighten or extend only the narrow records that cannot express a required distinction today.
- Carry source-owned action id, immediate consequence, next observable state, reversibility and
  composition mode where the source actually knows them.
- Represent material effects and uncertainties as separate semantic values where the corpus proves a
  list is real; retain authored prose where qualification is necessary.
- Preserve accountable author, source reference, source fingerprint/freshness and evidence locator.
- Make decision continuation derive from recorded source transitions rather than copied brief prose.
- Generate/update TypeScript contracts through the existing binding seam.

## Acceptance

- Every consequential control in the corpus maps to an existing source operation.
- Projection tests prove an authored recommendation cannot create, remove or widen source actions.
- A stale authored brief cannot be paired with current action semantics without being identified as
  stale.
- Unknown consequence or availability remains unknown; it is never projected as safe, empty or
  complete.
- Ordinary chat requires no structured payload.
- No universal presentation entity, second action lifecycle or presentation database is introduced.

## Branch and purge

Before implementation, compare at least:

1. extending current per-source records and projection types; and
2. adding one generic owner-content envelope.

The expected prior is the first. T1 records why the selected shape fits the corpus and deletes the
losing branch rather than retaining adapters for both.

