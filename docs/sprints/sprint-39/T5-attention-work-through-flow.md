# T5 — Add Work Through This to the Existing Attention Flow

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Interface + Organization Intelligence + Control Plane  
**Depends on:** T3, T4, T6

## Outcome

The owner can turn an existing Attention item into a focused, work-linked conversation with the accountable lead without leaving the current cockpit or creating a new coordination surface.

## Work

- Replace or extend the eligible Attention conversation action with **Work through this with {responsible lead}**.
- Reuse the existing Attention focus state and Executive Rail. Keep the item summary, rationale, recommendation, evidence, consequences, and typed resolution actions visible.
- Launch or restore the scoped responsible-actor conversation with `attentionId` and `workId` context; do not create a second transcript or focus-session record.
- Prepare a compact first-frame context from canonical Attention, Work, evidence, and open decision state. Do not rely on the owner restating the problem.
- Support ordinary follow-up turns for goal refinement, option comparison, bounded investigation, and recommendation.
- Show in-flight input as `applied` only after harness acknowledgement; otherwise show it as queued for the next turn.
- Preserve the unresolved Attention state when the rail closes or the session completes, fails, or is cancelled.
- Use only the existing typed action to accept, request changes, direct, approve, reject, defer, or dismiss. Preserve normal work links and provenance on the resulting event/message/decision.
- Ensure resolved-elsewhere and stale-item races cannot implicitly reopen or overwrite a decision.
- Cover narrow and small-window layouts without introducing a new page or dashboard.

## Acceptance

- The owner can enter, leave, and resume the work-through conversation from the original Attention item.
- The responsible actor receives the correct item, work, evidence, and open-action context on a fresh session and after reconstruction.
- Messages appear once in the existing canonical conversation timeline.
- Conversation completion never resolves the Attention item.
- Every actual resolution passes through an existing typed owner action and retains work/decision provenance.
- Applied-versus-queued input matches transport acknowledgement under race, cancel, and turn-end cases.
- A resolved-elsewhere item cannot be reopened or overwritten by a late harness event.
- The experience works with Restless Managed, Claude Agent, and Codex through the configured coordination harness without a harness picker.

## Makes Deletable

- Any prototype Focus Session route, model, or panel.
- Duplicate Attention evidence inside a chat-only store.
- UI state that treats the last assistant message as owner approval.
- Harness-specific branches in the Attention components.
