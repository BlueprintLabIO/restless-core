-- A recurring schedule is a clock opportunity, not an instruction to replay
-- stale work after downtime. Make that recovery choice durable and visible.
ALTER TABLE schedules
  ADD COLUMN missed_policy TEXT NOT NULL DEFAULT 'catch_up_once',
  ADD COLUMN catch_up_grace_seconds BIGINT,
  ADD COLUMN last_missed_at TIMESTAMPTZ,
  ADD CONSTRAINT schedules_missed_policy_shape CHECK (
    missed_policy IN ('skip', 'catch_up_once')
    AND (catch_up_grace_seconds IS NULL OR catch_up_grace_seconds > 0)
    AND (missed_policy = 'catch_up_once' OR catch_up_grace_seconds IS NULL)
  );

-- Existing recurring schedules retain the old coalescing behaviour until an
-- owner or accountable lead chooses an explicit bounded policy.

ALTER TABLE schedule_occurrences
  ADD COLUMN disposition TEXT NOT NULL DEFAULT 'fired',
  ADD COLUMN detail TEXT,
  ADD CONSTRAINT schedule_occurrence_disposition CHECK (
    disposition IN ('fired', 'skipped')
  );
