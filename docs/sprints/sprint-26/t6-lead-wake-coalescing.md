# S26-T6 — Coalesce supervisory wakes

Wake a lead for a decision, not for every fact that happens beneath it.

**Observed friction:** the EXP-15 lead woke after progress and feedback events, spent roughly a model
turn each time and repeatedly concluded that Staff was still active. These wakes added cost and latency
without changing work.

**Layer:** OrgIntel.

**Deletion target:** unconditional per-event lead wake and repeated still-active narration.

## Scope

- Classify terminal Staff facts, blockers, authority/policy questions and material semantic choices as
  decision-worthy; retain progress as observable state without an automatic model wake.
- Coalesce causally related facts while a lead wake is queued or active.
- Let a lead subscribe to a declared checkpoint without polling.
- Deliver the latest Work/Attempt state and compact causal delta once.
- Record why each supervisory wake occurred and whether it changed a decision.

## Acceptance

- A fixture with 100 progress facts and one terminal completion causes one lead wake.
- A genuine blocker wakes promptly and is not hidden behind a batching timer.
- Duplicate terminal delivery is idempotent.
- The adversarial sprint run records zero paid lead turns whose only conclusion is “still active”.

