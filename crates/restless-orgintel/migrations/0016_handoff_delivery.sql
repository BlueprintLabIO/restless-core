-- S19-T1. Whether an assignee has actually been given a pending judgement is a
-- durable fact about that judgement. It was previously inferred by comparing
-- the handoff's creation/escalation time with the newest Exec `wake` event of
-- any kind, so one unrelated wake moved the watermark past the handoff and it
-- never triggered a wake again.
--
-- This is a delivery record, not a delivery lifecycle: one nullable timestamp,
-- written only by a turn that actually carried the handoff in its context, and
-- cleared whenever the handoff is reassigned or its prepared meaning changes.
-- It gates the trigger, never the context: a delivered handoff that is still
-- pending remains in its assignee's context and in `blocked-on-a-person`.
ALTER TABLE owner_handoffs
  ADD COLUMN delivered_at TIMESTAMPTZ;
