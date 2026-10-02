ALTER TABLE schedules ADD COLUMN paused_at timestamptz;
CREATE INDEX schedules_active_due ON schedules (fire_at)
    WHERE fired_at IS NULL AND cancelled_at IS NULL AND paused_at IS NULL;
