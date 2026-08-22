-- One owner-facing conversation per durable actor, with one movable working
-- context boundary. This is a cursor over messages, not a thread entity or a
-- conversation lifecycle: history remains intact and the next actor wake may
-- selectively carry only messages newer than the cursor.
ALTER TABLE actors
  ADD COLUMN conversation_focus_after_message_id BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN conversation_focus_started_at TIMESTAMPTZ;
