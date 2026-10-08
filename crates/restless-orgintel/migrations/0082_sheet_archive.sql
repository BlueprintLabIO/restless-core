-- A sheet the owner set aside leaves the Library list and can be restored, as an archived document can.
ALTER TABLE native_sheets ADD COLUMN archived_at timestamptz;
