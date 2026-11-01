-- One Inbox item per thing (S109 B3).
--
-- A new collaboration request on a document that already has one open for the same person
-- amends that request instead of raising a second item. Existing duplicates are merged: the
-- oldest open request stays, so it keeps its place in the Inbox, and takes the newest words and
-- requester; the later ones are closed as superseded by it. Nothing is deleted.
ALTER TABLE native_document_collaboration_requests
  ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
  ADD COLUMN amended_at TIMESTAMPTZ,
  ADD COLUMN amended_by_key UUID,
  ADD COLUMN superseded_by UUID REFERENCES native_document_collaboration_requests(id);

CREATE TEMP TABLE open_collaboration_duplicates ON COMMIT DROP AS
SELECT id,
  first_value(id) OVER thing AS keep_id,
  last_value(summary) OVER thing AS latest_summary,
  last_value(requested_by_actor_id) OVER thing AS latest_by,
  count(*) OVER thing AS requests
FROM native_document_collaboration_requests
WHERE resolved_at IS NULL
WINDOW thing AS (
  PARTITION BY document_id, requested_owner_actor_id ORDER BY created_at, id
  ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
);

UPDATE native_document_collaboration_requests request
SET resolved_at = now(),
    resolved_by_actor_id = request.requested_by_actor_id,
    superseded_by = duplicate.keep_id
FROM open_collaboration_duplicates duplicate
WHERE request.id = duplicate.id AND duplicate.id <> duplicate.keep_id;

UPDATE native_document_collaboration_requests request
SET summary = duplicate.latest_summary,
    requested_by_actor_id = duplicate.latest_by,
    revision = duplicate.requests,
    amended_at = now()
FROM open_collaboration_duplicates duplicate
WHERE request.id = duplicate.id AND duplicate.id = duplicate.keep_id AND duplicate.requests > 1;

CREATE UNIQUE INDEX native_document_one_open_collaboration_idx
  ON native_document_collaboration_requests (document_id, requested_owner_actor_id)
  WHERE resolved_at IS NULL;

-- FYI notices (a change to something a person follows, a share) leave the Inbox for a short daily
-- digest in the person's Exec chat. This is how far each person's digest has reached.
CREATE TABLE for_you_notice_digests (
  actor_id TEXT PRIMARY KEY REFERENCES actors(id) ON DELETE CASCADE,
  through TIMESTAMPTZ NOT NULL,
  posted_at TIMESTAMPTZ,
  message_id BIGINT
);
