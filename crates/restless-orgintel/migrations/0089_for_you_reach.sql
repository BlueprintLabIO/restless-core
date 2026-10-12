-- For you reaches what Notion's inbox does (S65 completeness): being shared on a document or
-- sheet, being asked to review, replies in Room threads you are in, and sheets you follow. It is
-- still assembled from the objects themselves; the new state is per-person marks only.

-- A mark may also name one Room thread by its Room and root message: room-thread:<room>:<id>.
ALTER TABLE for_you_reads DROP CONSTRAINT for_you_reads_item_key_check;
ALTER TABLE for_you_reads ADD CONSTRAINT for_you_reads_item_key_check
  CHECK (item_key ~ '^[a-z-]{1,24}:[0-9a-f-]{36}(:[0-9]{1,19})?$');

-- A person may mark an item unread again (it stays unread until they open it), or archive it
-- (it leaves their list until something new happens there).
ALTER TABLE for_you_reads
  ADD COLUMN marked_unread BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN archived_at TIMESTAMPTZ;

-- The owner Inbox's seen and snooze marks follow the person across devices. Keyed by the
-- Attention item's own id; `seen_stamp` is the brief the person last opened, so a re-briefed
-- item reads as new again.
CREATE TABLE inbox_marks (
  actor_id TEXT NOT NULL REFERENCES actors(id),
  item_id TEXT NOT NULL CHECK (char_length(item_id) BETWEEN 1 AND 512),
  seen_stamp TEXT CHECK (char_length(seen_stamp) <= 64),
  snoozed_until TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (actor_id, item_id)
);

-- Who added someone to a sheet, and when, as documents already record. Grants made before this
-- have neither and never announce themselves.
ALTER TABLE native_sheet_participants
  ADD COLUMN IF NOT EXISTS added_by_actor_id TEXT REFERENCES actors(id),
  ADD COLUMN IF NOT EXISTS added_at TIMESTAMPTZ;

-- Following a sheet is implicit (making or editing it), with an explicit choice winning, as for
-- documents.
CREATE TABLE native_sheet_follows (
  sheet_id UUID NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  following BOOLEAN NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (sheet_id, actor_id)
);
CREATE INDEX native_sheet_messages_actor_idx
  ON native_sheet_messages (actor_id, sheet_id, accepted_at);
CREATE INDEX native_sheet_messages_accepted_idx
  ON native_sheet_messages (sheet_id, accepted_at);
