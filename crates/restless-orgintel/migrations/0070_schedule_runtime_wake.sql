-- A schedule may explicitly authorize waking its sleeping Runtime. Existing
-- schedules remain inert until their owner opts in.
ALTER TABLE schedules
  ADD COLUMN wake_runtime BOOLEAN NOT NULL DEFAULT false;
