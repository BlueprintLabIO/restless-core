-- Exec checks in with the owner about once an hour while they engage (13 November 2026). An
-- owner who does not want that turns it off with their other notification choices; no row, or
-- true, means on.
ALTER TABLE notification_preferences
  ADD COLUMN exec_checkins boolean NOT NULL DEFAULT true;
