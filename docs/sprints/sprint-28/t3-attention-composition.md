# S28-T3 — Render Attention as the decision it contains

**Layer:** Owner cockpit.

**Serves:** Sprint 28 criteria 1–6 and 11.

**Depends on:** S28-T1 and S28-T2.

**Observed friction:** The focused Attention folio presents five prose regions at similar weight,
while action consequences are often visible only through hover metadata. The owner reads an essay to
find a decision the source already represents.

**Makes deletable:** The wall-of-text folio, tooltip-only action consequences and category labels that
do not change the interaction.

## Outcome

Each supported Attention kind has a clear first-screen composition: owner-facing headline, concise
trigger/effects, accountable recommendation, real controls with visible consequences, waiting state,
material uncertainty and one evidence entry point.

## Scope

- Build from existing Restless primitives, typography, semantic colour and spacing.
- Render material effects as a list only when they are separate source-backed facts.
- Show the recommendation beside the real decision without styling authored judgement as source fact.
- Display immediate consequence and next observable state with each consequential choice.
- Use category-appropriate actions for approval, decision, review and human step.
- Implement multi-select only if T0 admitted a real independently composable source case.
- Preserve native ReviewTarget priority, evidence disclosure, author/source attribution, deadline and
  material uncertainty.
- Preserve keyboard operation, visible focus, narrow-screen order and non-hover access to every
  consequence.

## Acceptance

- Corpus component tests assert that controls come from source action ids, not parsed copy.
- An approval cannot render as an outcome acceptance; conversational feedback cannot grant authority.
- A reader can predict every control's immediate consequence without a tooltip.
- The first view contains no repeated situation/recommendation/requested-action paragraph when one
  semantic value suffices.
- Long truthful context can expand without hiding the decision or truncating evidence.
- Desktop and 390px-wide captures show no overflow and retain logical focus order.
- The visual review compares the live composition with the relevant source-first references named in
  `docs/FRONTEND_DESIGN_REFERENCES.md` without importing a second identity.

