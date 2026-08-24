-- S12-T3: a failed cognitive process can leave useful Runtime work behind.
-- Keep one recovery notice attached to the immutable Attempt so repeated
-- reconciliation cannot manufacture duplicate lead messages or references.
-- This is an OrgIntel coordination pointer, not a process database or
-- artifact-custody lifecycle.

ALTER TABLE work_attempts
  ADD COLUMN recovery_message_id BIGINT REFERENCES messages(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX work_attempts_recovery_message
  ON work_attempts (recovery_message_id)
  WHERE recovery_message_id IS NOT NULL;
