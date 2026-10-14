-- A calendar schedule runs at a local time on chosen days. NULL keeps the original
-- meaning of a `weekdays` recurrence, Monday to Friday; otherwise `daily`,
-- `weekly:<ISO weekdays>` (1 = Monday … 7 = Sunday) or `monthly:<day>` (a day past
-- the end of a short month runs on its last day).
ALTER TABLE schedules ADD COLUMN calendar_days TEXT;
ALTER TABLE schedules ADD CONSTRAINT schedules_calendar_days_shape CHECK (
    calendar_days IS NULL
    OR (recurrence = 'weekdays'
        AND calendar_days ~ '^(daily|weekly:[1-7](,[1-7]){0,6}|monthly:([1-9]|[12][0-9]|3[01]))$')
);

-- The same check-in on different days is a different schedule.
DROP INDEX one_live_recurring_schedule;
CREATE UNIQUE INDEX one_live_recurring_schedule
  ON schedules (actor_id, recurrence, timezone, local_time, reason, COALESCE(calendar_days, ''))
  WHERE recurrence IS NOT NULL AND cancelled_at IS NULL;
