-- Every due schedule now wakes a sleeping company computer, so the per-schedule
-- opt-in is gone. A computer the owner stopped is never woken by a schedule.
ALTER TABLE schedules DROP COLUMN IF EXISTS wake_runtime;
