-- Sprint 65 completeness: a comment is its author's to edit or take back, a resolved thread can
-- be reopened, and someone who cannot open a document (or may only read it) can ask for more.

-- The comment row is the current comment. Editing replaces its content and says when; deleting
-- hides it everywhere and keeps the row, so replies, reactions and receipts that name it resolve.
-- Who wrote it, where, and when stay fixed, and a deleted comment stays deleted.
ALTER TABLE native_document_comments
  ADD COLUMN edited_at TIMESTAMPTZ,
  ADD COLUMN deleted_at TIMESTAMPTZ;

CREATE FUNCTION orgintel_guard_native_document_comment_mutation()
RETURNS trigger AS $$
BEGIN
  IF NEW.id <> OLD.id
    OR NEW.document_id <> OLD.document_id
    OR NEW.thread_id <> OLD.thread_id
    OR NEW.reply_to_comment_id IS DISTINCT FROM OLD.reply_to_comment_id
    OR NEW.author_actor_id <> OLD.author_actor_id
    OR NEW.created_at <> OLD.created_at
    OR OLD.deleted_at IS NOT NULL THEN
    RAISE EXCEPTION 'native document comment identity is immutable';
  END IF;
  IF NEW.edited_at IS NOT DISTINCT FROM OLD.edited_at
    AND NEW.deleted_at IS NOT DISTINCT FROM OLD.deleted_at THEN
    RAISE EXCEPTION 'a native document comment changes only by an edit or a delete';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER native_document_comments_are_immutable ON native_document_comments;
CREATE TRIGGER native_document_comment_identity_is_immutable
  BEFORE UPDATE ON native_document_comments
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_comment_mutation();

-- A thread moves open -> resolved and, now, resolved -> open. Its anchor and author never change.
CREATE OR REPLACE FUNCTION orgintel_guard_native_document_comment_thread_mutation()
RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'native document comment thread history is immutable';
  END IF;
  IF NEW.id <> OLD.id
    OR NEW.document_id <> OLD.document_id
    OR NEW.anchored_version_id <> OLD.anchored_version_id
    OR NEW.block_id IS DISTINCT FROM OLD.block_id
    OR NEW.created_by_actor_id <> OLD.created_by_actor_id
    OR NEW.created_at <> OLD.created_at THEN
    RAISE EXCEPTION 'native document comment thread evidence is immutable';
  END IF;
  IF NEW.version <> OLD.version + 1 THEN
    RAISE EXCEPTION 'invalid native document comment thread transition';
  END IF;
  IF OLD.status = 'open' AND NEW.status = 'resolved'
    AND NEW.resolved_by_actor_id IS NOT NULL AND NEW.resolved_at IS NOT NULL THEN
    RETURN NEW;
  END IF;
  IF OLD.status = 'resolved' AND NEW.status = 'open'
    AND NEW.resolved_by_actor_id IS NULL AND NEW.resolved_at IS NULL THEN
    RETURN NEW;
  END IF;
  RAISE EXCEPTION 'invalid native document comment thread transition';
END;
$$ LANGUAGE plpgsql;

-- One open ask per person and document: to open it at all, or to comment on or edit what they
-- may only read. It reaches the people who manage the document's sharing in For you. It is
-- answered by giving that access, which closes it, or by dismissing it. Asking again within a
-- day of an answer raises nothing unless it asks for more; after that it asks anew.
CREATE TABLE native_document_access_requests (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  access native_document_access NOT NULL,
  requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  closed_at TIMESTAMPTZ,
  closed_by_actor_id TEXT REFERENCES actors(id),
  UNIQUE (document_id, actor_id),
  CHECK ((closed_at IS NULL) = (closed_by_actor_id IS NULL))
);
CREATE INDEX native_document_access_requests_open_idx
  ON native_document_access_requests (document_id) WHERE closed_at IS NULL;
