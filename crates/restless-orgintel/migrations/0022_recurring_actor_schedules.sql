-- A recurring schedule is still only a durable time fact addressed to an
-- actor. It never stores a command or implies that production must happen.
ALTER TABLE schedules
  ADD COLUMN recurrence TEXT,
  ADD COLUMN timezone TEXT,
  ADD COLUMN local_time TIME,
  ADD COLUMN last_fired_at TIMESTAMPTZ,
  ADD CONSTRAINT schedules_recurrence_shape CHECK (
    (recurrence IS NULL AND timezone IS NULL AND local_time IS NULL)
    OR
    (recurrence = 'weekdays' AND timezone IS NOT NULL AND local_time IS NOT NULL AND work_id IS NULL)
  );

-- Repeating the same owner command returns the existing live schedule rather
-- than creating two actors wakes for the same operating cadence.
CREATE UNIQUE INDEX one_live_recurring_schedule
  ON schedules (actor_id, recurrence, timezone, local_time, reason)
  WHERE recurrence IS NOT NULL AND cancelled_at IS NULL;

-- This is recoverable delivery history, not an Authority ledger. The unique
-- occurrence key closes restart and concurrent-claim duplication.
CREATE TABLE schedule_occurrences (
  schedule_id   UUID NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
  scheduled_for TIMESTAMPTZ NOT NULL,
  fired_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (schedule_id, scheduled_for)
);
