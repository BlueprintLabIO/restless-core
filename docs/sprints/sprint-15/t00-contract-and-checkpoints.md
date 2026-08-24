# S15-T0 — Freeze trust-boundary contract and checkpoint baseline

**Layer:** Cross-layer contract plus evaluation.

**Observed friction served:** `main.rs` still describes caller-supplied company identity as an accepted
single-operator risk whose expiry was “before any real external effect”; real effects now exist. Broad
uncommitted Sprint 14 work also makes recovery progress vulnerable to accidental loss.

## Outcome

The sprint begins from one explicit capability-boundary contract and saves each independently verified
slice in a narrow commit.

## Acceptance

- Name the listener, model and ceiling defects; classify their risks and non-goals.
- Record the rejected network-only and full-identity-service alternatives before implementation.
- Add repository guidance for verified commits and owner-authorised pushes without weakening the
  existing no-unasked-push rule.
- Preserve pre-existing dirty work: do not silently stage it as part of this ticket.

## Deletion target

Expired “trusted as sent” rationale and ambiguous progress-saving practice.
