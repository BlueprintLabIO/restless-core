-- Native Docs company semantics: bounded comment threads, explicit review
-- acceptance, and attributed revision proposals. Structured named versions
-- remain the only accepted-body history; none of these tables is a second
-- live editing path.

CREATE TYPE native_document_comment_status AS ENUM ('open', 'resolved');
CREATE TYPE native_document_review_status AS ENUM ('requested', 'accepted', 'stale');
CREATE TYPE native_document_revision_scope AS ENUM ('whole_document', 'block');
CREATE TYPE native_document_revision_status AS ENUM (
  'proposed',
  'accepted',
  'rejected',
  'stale'
);

CREATE TABLE native_document_comment_threads (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  anchored_version_id UUID NOT NULL,
  block_id TEXT,
  status native_document_comment_status NOT NULL DEFAULT 'open',
  created_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_by_actor_id TEXT REFERENCES actors(id),
  resolved_at TIMESTAMPTZ,
  version BIGINT NOT NULL DEFAULT 1 CHECK (version >= 1),
  UNIQUE (document_id, id),
  FOREIGN KEY (document_id, anchored_version_id)
    REFERENCES native_document_versions(document_id, id),
  CHECK (
    block_id IS NULL
    OR (
      char_length(block_id) BETWEEN 1 AND 128
      AND block_id ~ '^[A-Za-z0-9][A-Za-z0-9._:-]*$'
    )
  ),
  CHECK (
    (status = 'open' AND resolved_by_actor_id IS NULL AND resolved_at IS NULL)
    OR
    (status = 'resolved' AND resolved_by_actor_id IS NOT NULL AND resolved_at IS NOT NULL)
  )
);

CREATE TABLE native_document_comments (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL,
  thread_id UUID NOT NULL,
  reply_to_comment_id UUID,
  author_actor_id TEXT NOT NULL REFERENCES actors(id),
  content_json JSONB NOT NULL CHECK (jsonb_typeof(content_json) = 'object'),
  plain_text TEXT NOT NULL CHECK (char_length(plain_text) BETWEEN 1 AND 8000),
  content_hash TEXT NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
  mentioned_actor_ids TEXT[] NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (thread_id, id),
  FOREIGN KEY (document_id, thread_id)
    REFERENCES native_document_comment_threads(document_id, id) ON DELETE CASCADE,
  FOREIGN KEY (thread_id, reply_to_comment_id)
    REFERENCES native_document_comments(thread_id, id),
  CHECK (reply_to_comment_id IS NULL OR reply_to_comment_id <> id),
  CHECK (cardinality(mentioned_actor_ids) <= 32)
);

CREATE TABLE native_document_reviews (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  requested_version_id UUID NOT NULL,
  requested_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  summary TEXT NOT NULL CHECK (char_length(btrim(summary)) BETWEEN 1 AND 1000),
  status native_document_review_status NOT NULL DEFAULT 'requested',
  accepted_version_id UUID,
  accepted_by_actor_id TEXT REFERENCES actors(id),
  accepted_at TIMESTAMPTZ,
  material_unresolved_thread_ids UUID[],
  stale_against_version_id UUID,
  stale_detected_by_actor_id TEXT REFERENCES actors(id),
  stale_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  version BIGINT NOT NULL DEFAULT 1 CHECK (version >= 1),
  UNIQUE (document_id, id),
  FOREIGN KEY (document_id, requested_version_id)
    REFERENCES native_document_versions(document_id, id),
  FOREIGN KEY (document_id, accepted_version_id)
    REFERENCES native_document_versions(document_id, id),
  FOREIGN KEY (document_id, stale_against_version_id)
    REFERENCES native_document_versions(document_id, id),
  CHECK (
    (status = 'requested' AND accepted_version_id IS NULL
      AND accepted_by_actor_id IS NULL AND accepted_at IS NULL
      AND material_unresolved_thread_ids IS NULL
      AND stale_against_version_id IS NULL
      AND stale_detected_by_actor_id IS NULL AND stale_at IS NULL)
    OR
    (status = 'accepted' AND accepted_version_id IS NOT NULL
      AND accepted_by_actor_id IS NOT NULL AND accepted_at IS NOT NULL
      AND material_unresolved_thread_ids IS NOT NULL
      AND stale_against_version_id IS NULL
      AND stale_detected_by_actor_id IS NULL AND stale_at IS NULL
      AND cardinality(material_unresolved_thread_ids) <= 256)
    OR
    (status = 'stale' AND accepted_version_id IS NULL
      AND accepted_by_actor_id IS NULL AND accepted_at IS NULL
      AND material_unresolved_thread_ids IS NULL
      AND stale_against_version_id IS NOT NULL
      AND stale_detected_by_actor_id IS NOT NULL AND stale_at IS NOT NULL)
  )
);

CREATE TABLE native_document_revision_proposals (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  base_version_id UUID NOT NULL,
  scope native_document_revision_scope NOT NULL,
  block_id TEXT,
  proposed_content_json JSONB NOT NULL CHECK (jsonb_typeof(proposed_content_json) = 'object'),
  proposed_plain_text TEXT NOT NULL,
  proposed_content_hash TEXT NOT NULL CHECK (proposed_content_hash ~ '^[0-9a-f]{64}$'),
  summary TEXT NOT NULL CHECK (char_length(btrim(summary)) BETWEEN 1 AND 1000),
  proposed_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  status native_document_revision_status NOT NULL DEFAULT 'proposed',
  accepted_version_id UUID,
  resolved_by_actor_id TEXT REFERENCES actors(id),
  resolution_summary TEXT,
  resolved_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  version BIGINT NOT NULL DEFAULT 1 CHECK (version >= 1),
  UNIQUE (document_id, id),
  FOREIGN KEY (document_id, base_version_id)
    REFERENCES native_document_versions(document_id, id),
  FOREIGN KEY (document_id, accepted_version_id)
    REFERENCES native_document_versions(document_id, id),
  CHECK (
    (scope = 'whole_document' AND block_id IS NULL)
    OR
    (scope = 'block' AND block_id IS NOT NULL
      AND char_length(block_id) BETWEEN 1 AND 128
      AND block_id ~ '^[A-Za-z0-9][A-Za-z0-9._:-]*$')
  ),
  CHECK (
    (status = 'proposed' AND accepted_version_id IS NULL
      AND resolved_by_actor_id IS NULL AND resolution_summary IS NULL
      AND resolved_at IS NULL)
    OR
    (status = 'accepted' AND accepted_version_id IS NOT NULL
      AND resolved_by_actor_id IS NOT NULL AND resolution_summary IS NOT NULL
      AND resolved_at IS NOT NULL)
    OR
    (status IN ('rejected', 'stale') AND accepted_version_id IS NULL
      AND resolved_by_actor_id IS NOT NULL AND resolution_summary IS NOT NULL
      AND resolved_at IS NOT NULL)
  )
);

-- One command key has one meaning inside a company schema. The bounded receipt
-- makes a committed response reconstructable after transport loss without
-- storing a second JSON projection of the result.
CREATE TABLE native_document_command_receipts (
  command_id UUID PRIMARY KEY,
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  operation TEXT NOT NULL CHECK (operation IN (
    'comment_thread_create',
    'comment_reply',
    'comment_thread_resolve',
    'review_request',
    'review_accept',
    'revision_propose',
    'revision_accept',
    'revision_reject'
  )),
  request_fingerprint TEXT NOT NULL CHECK (request_fingerprint ~ '^[0-9a-f]{64}$'),
  result_id UUID NOT NULL,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX native_document_comment_threads_page_idx
  ON native_document_comment_threads (document_id, created_at, id);
CREATE INDEX native_document_comments_page_idx
  ON native_document_comments (thread_id, created_at, id);
CREATE INDEX native_document_reviews_document_idx
  ON native_document_reviews (document_id, created_at, id);
CREATE UNIQUE INDEX native_document_one_requested_review_idx
  ON native_document_reviews (document_id) WHERE status = 'requested';
CREATE INDEX native_document_revision_proposals_document_idx
  ON native_document_revision_proposals (document_id, created_at, id);

-- Comment text and command receipts are append-only evidence. Thread/review/
-- proposal lifecycle changes remain explicit, versioned state transitions.
CREATE FUNCTION orgintel_reject_native_document_comment_mutation()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'native document comments are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_comments_are_immutable
  BEFORE UPDATE OR DELETE ON native_document_comments
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_comment_mutation();

CREATE FUNCTION orgintel_guard_native_document_comment_thread_mutation()
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
  IF OLD.status <> 'open'
    OR NEW.status <> 'resolved'
    OR NEW.version <> OLD.version + 1
    OR NEW.resolved_by_actor_id IS NULL
    OR NEW.resolved_at IS NULL THEN
    RAISE EXCEPTION 'invalid native document comment thread transition';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_comment_thread_evidence_is_immutable
  BEFORE UPDATE OR DELETE ON native_document_comment_threads
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_comment_thread_mutation();

CREATE FUNCTION orgintel_reject_native_document_command_receipt_mutation()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'native document command receipts are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_command_receipts_are_immutable
  BEFORE UPDATE OR DELETE ON native_document_command_receipts
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_command_receipt_mutation();

CREATE FUNCTION orgintel_guard_native_document_review_mutation()
RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'native document review history is immutable';
  END IF;
  IF NEW.id <> OLD.id
    OR NEW.document_id <> OLD.document_id
    OR NEW.requested_version_id <> OLD.requested_version_id
    OR NEW.requested_by_actor_id <> OLD.requested_by_actor_id
    OR NEW.summary <> OLD.summary
    OR NEW.created_at <> OLD.created_at THEN
    RAISE EXCEPTION 'native document review request evidence is immutable';
  END IF;
  IF OLD.status <> 'requested'
    OR NEW.status NOT IN ('accepted', 'stale')
    OR NEW.version <> OLD.version + 1 THEN
    RAISE EXCEPTION 'invalid native document review transition';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_review_request_is_immutable
  BEFORE UPDATE OR DELETE ON native_document_reviews
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_review_mutation();

CREATE FUNCTION orgintel_guard_native_document_revision_proposal_mutation()
RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'native document revision proposal history is immutable';
  END IF;
  IF NEW.id <> OLD.id
    OR NEW.document_id <> OLD.document_id
    OR NEW.base_version_id <> OLD.base_version_id
    OR NEW.scope <> OLD.scope
    OR NEW.block_id IS DISTINCT FROM OLD.block_id
    OR NEW.proposed_content_json <> OLD.proposed_content_json
    OR NEW.proposed_plain_text <> OLD.proposed_plain_text
    OR NEW.proposed_content_hash <> OLD.proposed_content_hash
    OR NEW.summary <> OLD.summary
    OR NEW.proposed_by_actor_id <> OLD.proposed_by_actor_id
    OR NEW.created_at <> OLD.created_at THEN
    RAISE EXCEPTION 'native document revision proposal evidence is immutable';
  END IF;
  IF OLD.status <> 'proposed'
    OR NEW.status NOT IN ('accepted', 'rejected', 'stale')
    OR NEW.version <> OLD.version + 1
    OR NEW.resolved_by_actor_id IS NULL
    OR NEW.resolution_summary IS NULL
    OR NEW.resolved_at IS NULL THEN
    RAISE EXCEPTION 'invalid native document revision proposal transition';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_revision_proposal_is_immutable
  BEFORE UPDATE OR DELETE ON native_document_revision_proposals
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_revision_proposal_mutation();
