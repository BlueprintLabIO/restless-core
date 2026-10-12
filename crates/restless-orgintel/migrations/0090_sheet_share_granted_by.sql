-- Who added someone to a sheet and when, so its share panel says who granted access, as a
-- document's does. Rows added before this have no record of it and stay unattributed.
ALTER TABLE native_sheet_participants
  ADD COLUMN added_by_actor_id text REFERENCES actors(id),
  ADD COLUMN added_at timestamptz;
