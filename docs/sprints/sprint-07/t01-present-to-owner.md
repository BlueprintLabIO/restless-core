# S07-T1 · Shared present-to-owner skill and accountable preparation

**Layer:** OrgIntel + Runtime context.  
**Serves:** Sprint 07 criteria 2, 3, 7, 8 and 9.  
**Depends on:** S06-T5.  
**Observed friction:** accountable leads currently write detailed internal reports into
`prepared_state`; no final step owns selection, plain-language consequence or recommendation.  
**Makes deletable:** the instruction that treats raw `--prepared` prose as the owner's primary
reading surface.

## Outcome

The accountable lead, or Exec at its bounded altitude, uses one shared presentation skill to write a
stable owner brief against the exact handoff source state. Runtime supplies the same skill text to
both actor contexts. OrgIntel records the authored result; the browser performs no model call.

## Scope

- Add one shared `present-to-owner` instruction used by lead and Exec contexts.
- Add the smallest typed payload the existing handoff cannot represent without parsing prose:
  kind, headline, situation, impact, recommendation, no-action consequence, optional uncertainty
  and deadline.
- Record author, authored time and a deterministic fingerprint of the handoff source snapshot.
- Permit the Work owner, its accountable lead or Exec to prepare/refresh it; preserve attribution.
- Keep `prepared_state`, artifacts, gates and ReviewTarget as source evidence rather than deleting or
  copying them into the brief.

## Verification

- The same skill source is present in both lead and Exec prompts.
- Preparing a brief persists its author and source fingerprint across an OrgIntel reconnect.
- An unrelated actor cannot replace it.
- Changing source fields leaves the older brief detectable as stale until an attributed refresh.

## Risks

- **Brief becomes a second source of truth — guarded:** it explains a source snapshot and stores its
  fingerprint; operational state and evidence remain in their owning planes.
- **Template replaces judgement — accepted:** structure is required for stable rendering, but copy
  length and domain language are editorial defaults rather than database rules.

