-- A terminal Work fact is owed to the accountable lead only after the
-- Attempt, artifacts and gates have committed. Keep that tiny callback as a
-- recoverable outbox bit on the Attempt itself: live completion flushes it
-- immediately, while daemon restart can flush the same fact exactly once.
-- Existing terminal Attempts predate this contract and are deliberately not
-- backfilled into a burst of historical supervisor wakes.

ALTER TABLE work_attempts
  ADD COLUMN supervisor_notice_owed BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN supervisor_notice_message_id BIGINT REFERENCES messages(id);

CREATE INDEX work_attempts_supervisor_notice_owed
  ON work_attempts (finished_at, id)
  WHERE supervisor_notice_owed;
