-- Sharing with someone invited to the company who has not joined yet (S66). The share waits under
-- their email, as the invitation holds it, and is given when they first enter, by whoever shared it.
CREATE TABLE pending_shares (
  id uuid PRIMARY KEY,
  email text NOT NULL CHECK (
    email = lower(btrim(email)) AND length(email) BETWEEN 3 AND 320 AND position('@' IN email) > 1
  ),
  document_id uuid REFERENCES native_documents(id) ON DELETE CASCADE,
  sheet_id uuid REFERENCES native_sheets(id) ON DELETE CASCADE,
  access text NOT NULL CHECK (access IN ('read', 'comment', 'edit')),
  invited_by_actor_id text NOT NULL REFERENCES actors(id),
  invited_at timestamptz NOT NULL DEFAULT now(),
  CHECK ((document_id IS NULL) <> (sheet_id IS NULL)),
  CHECK (sheet_id IS NULL OR access IN ('read', 'edit'))
);
CREATE UNIQUE INDEX pending_shares_document_idx
  ON pending_shares (document_id, email) WHERE document_id IS NOT NULL;
CREATE UNIQUE INDEX pending_shares_sheet_idx
  ON pending_shares (sheet_id, email) WHERE sheet_id IS NOT NULL;
CREATE INDEX pending_shares_email_idx ON pending_shares (email);
