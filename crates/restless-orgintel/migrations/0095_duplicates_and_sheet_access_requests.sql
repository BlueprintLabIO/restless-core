-- Which document a duplicate was copied from (S68), so the copy shows the images it was made with:
-- an image placed by an editor of the original is placed in the copy too. Set only by Duplicate.
ALTER TABLE native_documents
  ADD COLUMN IF NOT EXISTS duplicated_from UUID REFERENCES native_documents(id) ON DELETE SET NULL;

-- Asking for a sheet, as for a document (S68): someone who cannot open it, or can only view it,
-- asks whoever may share it. One open ask per person; granting or setting it aside closes it.
CREATE TABLE IF NOT EXISTS native_sheet_access_requests (
  id UUID PRIMARY KEY,
  sheet_id UUID NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  access TEXT NOT NULL CHECK (access IN ('read', 'edit')),
  requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  closed_at TIMESTAMPTZ,
  closed_by_actor_id TEXT REFERENCES actors(id),
  UNIQUE (sheet_id, actor_id),
  CHECK ((closed_at IS NULL) = (closed_by_actor_id IS NULL))
);
CREATE INDEX IF NOT EXISTS native_sheet_access_requests_open_idx
  ON native_sheet_access_requests (sheet_id) WHERE closed_at IS NULL;
