# S07-T4 · One narrative, stable across main surface and conversation

**Layer:** Owner surface + OrgIntel messages.  
**Serves:** Sprint 07 criteria 5, 6 and 9.  
**Depends on:** S07-T2, S07-T3.  
**Observed friction:** focused review repeats the selected title and prepared recommendation inside
the conversation rail, creating a second account of the same item.  
**Makes deletable:** the rail's duplicate review-context block and seeded pseudo-message.

## Outcome

The main surface owns the prepared story and decision. The rail identifies the accountable actor and
holds the durable Work-scoped conversation only. Previously delivered messages and streamed text stay
stable while new activity arrives. Discussion never implies a decision.

## Scope

- Remove duplicated selected-item narrative from the rail and browser handover thread.
- Keep compact hold-to-accept and back controls for a focused outcome review.
- Use a short composer placeholder and plain empty state inviting a question or revision feedback.
- Preserve the existing SSE snapshot stream and append-only visible transcript behaviour.

## Verification

- The brief appears once, on the main outcome/attention surface.
- The rail contains only identity, actual messages, live activity and composer controls.
- Sending a message leaves the handoff pending until an explicit source-owned action occurs.
- Earlier message/text blocks remain mounted while thinking, tool activity and response snapshots
  update.

