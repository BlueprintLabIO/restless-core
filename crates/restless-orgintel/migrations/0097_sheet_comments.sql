-- Comments on sheets (S74), as on documents: a thread on a cell, replies, mentions, resolve and
-- reopen. A sheet gains the comment level documents have: someone who may comment reads the grid,
-- cannot change it, and may take part in its threads. Every level that already exists keeps what
-- it allowed.
--
-- The checks that held sheets to view and edit are found by what they say rather than by name,
-- since the older ones were never named.
DO $$
DECLARE
  found record;
BEGIN
  FOR found IN
    SELECT c.conrelid::regclass::text AS tbl, c.conname
    FROM pg_constraint c
    WHERE c.contype = 'c'
      AND c.conrelid IN ('native_sheets'::regclass, 'native_sheet_participants'::regclass,
        'native_sheet_access_requests'::regclass, 'pending_shares'::regclass)
      AND (
        (c.conrelid = 'native_sheets'::regclass AND pg_get_constraintdef(c.oid) LIKE '%company_access%')
        OR (c.conrelid IN ('native_sheet_participants'::regclass, 'native_sheet_access_requests'::regclass)
          AND pg_get_constraintdef(c.oid) LIKE '%access%'
          AND pg_get_constraintdef(c.oid) NOT LIKE '%closed%')
        OR (c.conrelid = 'pending_shares'::regclass AND pg_get_constraintdef(c.oid) LIKE '%sheet_id IS NULL%'
          AND pg_get_constraintdef(c.oid) LIKE '%access%')
      )
  LOOP
    EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', found.tbl, found.conname);
  END LOOP;
END $$;

ALTER TABLE native_sheets ADD CONSTRAINT native_sheets_company_access_level
  CHECK (company_access IN ('read', 'comment', 'edit'));
ALTER TABLE native_sheet_participants ADD CONSTRAINT native_sheet_participants_access_level
  CHECK (access IN ('read', 'comment', 'edit'));
ALTER TABLE native_sheet_access_requests ADD CONSTRAINT native_sheet_access_requests_access_level
  CHECK (access IN ('read', 'comment', 'edit'));

-- A thread is on one cell of one tab, named as the grid names it (B3). The address stays where
-- it was written; a row inserted above it does not carry the thread down.
CREATE TABLE native_sheet_comment_threads (
  id UUID PRIMARY KEY,
  sheet_id UUID NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  worksheet_id TEXT NOT NULL CHECK (char_length(worksheet_id) BETWEEN 1 AND 128),
  cell TEXT NOT NULL CHECK (cell ~ '^[A-Z]{1,3}[1-9][0-9]{0,6}$'),
  created_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_by_actor_id TEXT REFERENCES actors(id),
  resolved_at TIMESTAMPTZ,
  UNIQUE (sheet_id, id),
  CHECK ((resolved_at IS NULL) = (resolved_by_actor_id IS NULL))
);
CREATE INDEX native_sheet_comment_threads_sheet_idx
  ON native_sheet_comment_threads (sheet_id, created_at DESC);

CREATE TABLE native_sheet_comments (
  id UUID PRIMARY KEY,
  sheet_id UUID NOT NULL,
  thread_id UUID NOT NULL,
  author_actor_id TEXT NOT NULL REFERENCES actors(id),
  body TEXT NOT NULL CHECK (char_length(btrim(body)) BETWEEN 1 AND 8000),
  mentioned_actor_ids TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(mentioned_actor_ids) <= 32),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  edited_at TIMESTAMPTZ,
  deleted_at TIMESTAMPTZ,
  FOREIGN KEY (sheet_id, thread_id)
    REFERENCES native_sheet_comment_threads (sheet_id, id) ON DELETE CASCADE
);
CREATE INDEX native_sheet_comments_thread_idx ON native_sheet_comments (thread_id, created_at);
CREATE INDEX native_sheet_comments_mentions_idx
  ON native_sheet_comments USING gin (mentioned_actor_ids);

-- Live for everyone who has the sheet or For you open, as document comments are (S70).
CREATE TRIGGER native_sheet_comment_threads_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_sheet_comment_threads
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_sheet_comments_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_sheet_comments
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
