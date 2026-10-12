-- For you (S65): a projection over mentions, comment threads and named versions that already
-- exist. The only new state is what each person has read, which documents they chose to follow or
-- leave, and who was editing a document when its live body was saved as an automatic version.

-- One row per person and item they opened. An item is unread while it has activity newer than
-- both this and the person's "all read through" cursor. Keys name the source object, never a copy.
CREATE TABLE for_you_reads (
  actor_id TEXT NOT NULL REFERENCES actors(id),
  item_key TEXT NOT NULL CHECK (item_key ~ '^[a-z-]{1,24}:[0-9a-f-]{36}$'),
  read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (actor_id, item_key)
);

CREATE TABLE for_you_read_cursors (
  actor_id TEXT PRIMARY KEY REFERENCES actors(id),
  read_through TIMESTAMPTZ NOT NULL
);

-- Following is implicit (creating, editing, commenting on or being mentioned in a document).
-- A row exists only once a person chooses explicitly, and their choice wins.
CREATE TABLE native_document_follows (
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  following BOOLEAN NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (document_id, actor_id)
);

-- Live edits are saved by the daemon as automatic versions. The people who held a write session
-- since the previous version are recorded beside that version, so "who changed it" survives the
-- short-lived session rows.
CREATE TABLE native_document_version_editors (
  document_id UUID NOT NULL,
  version_id UUID NOT NULL,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  PRIMARY KEY (version_id, actor_id),
  FOREIGN KEY (document_id, version_id)
    REFERENCES native_document_versions(document_id, id) ON DELETE CASCADE
);
CREATE INDEX native_document_version_editors_actor_idx
  ON native_document_version_editors (actor_id, document_id);
CREATE INDEX native_document_versions_author_idx
  ON native_document_versions (created_by_actor_id, document_id);
CREATE INDEX native_document_comments_author_idx
  ON native_document_comments (author_actor_id, document_id);
