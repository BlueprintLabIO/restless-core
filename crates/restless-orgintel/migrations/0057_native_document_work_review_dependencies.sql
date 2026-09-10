-- Bind a native-Document review of one immutable named version to exactly one
-- blocked Work node. The binding is explicit company state; accepting it may
-- resume only that Work, while requesting changes resumes the same Work with
-- exact feedback for its next Attempt. Resolution is replayed through the existing durable
-- native-Document command receipts.

DROP INDEX native_document_one_requested_review_idx;
ALTER TABLE native_document_reviews
  DROP CONSTRAINT native_document_reviews_check;

ALTER TYPE native_document_review_status RENAME TO native_document_review_status_v1;
CREATE TYPE native_document_review_status AS ENUM (
  'requested',
  'accepted',
  'changes_requested',
  'stale'
);
ALTER TABLE native_document_reviews
  ALTER COLUMN status DROP DEFAULT,
  ALTER COLUMN status TYPE native_document_review_status
    USING status::text::native_document_review_status,
  ALTER COLUMN status SET DEFAULT 'requested';
DROP TYPE native_document_review_status_v1;

ALTER TABLE native_document_reviews
  ADD CONSTRAINT native_document_reviews_resolution_check CHECK (
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
    (status = 'changes_requested' AND accepted_version_id IS NULL
      AND accepted_by_actor_id IS NULL AND accepted_at IS NULL
      AND material_unresolved_thread_ids IS NULL
      AND stale_against_version_id IS NULL
      AND stale_detected_by_actor_id IS NULL AND stale_at IS NULL)
    OR
    (status = 'stale' AND accepted_version_id IS NULL
      AND accepted_by_actor_id IS NULL AND accepted_at IS NULL
      AND material_unresolved_thread_ids IS NULL
      AND stale_against_version_id IS NOT NULL
      AND stale_detected_by_actor_id IS NOT NULL AND stale_at IS NOT NULL)
  );

CREATE UNIQUE INDEX native_document_one_requested_review_idx
  ON native_document_reviews (document_id) WHERE status = 'requested';

CREATE OR REPLACE FUNCTION orgintel_guard_native_document_review_mutation()
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
    OR NEW.status NOT IN ('accepted', 'changes_requested', 'stale')
    OR NEW.version <> OLD.version + 1 THEN
    RAISE EXCEPTION 'invalid native document review transition';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

ALTER TABLE native_document_reviews
  ADD CONSTRAINT native_document_reviews_document_id_id_requested_version_id_key
  UNIQUE (document_id, id, requested_version_id);

CREATE TYPE native_document_work_review_status AS ENUM (
  'pending',
  'accepted',
  'changes_requested',
  'stale'
);

CREATE TABLE native_document_work_review_dependencies (
  review_id UUID PRIMARY KEY,
  document_id UUID NOT NULL,
  requested_version_id UUID NOT NULL,
  work_id UUID NOT NULL REFERENCES work(id),
  attempt_id UUID NOT NULL REFERENCES work_attempts(id),
  work_revision BIGINT NOT NULL CHECK (work_revision > 0),
  reviewer_actor_id TEXT NOT NULL REFERENCES actors(id),
  status native_document_work_review_status NOT NULL DEFAULT 'pending',
  feedback TEXT,
  resolved_by_actor_id TEXT REFERENCES actors(id),
  resolved_at TIMESTAMPTZ,
  resumed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  FOREIGN KEY (document_id, review_id, requested_version_id)
    REFERENCES native_document_reviews(document_id, id, requested_version_id)
    ON DELETE CASCADE,
  CHECK (
    (status = 'pending' AND feedback IS NULL AND resolved_by_actor_id IS NULL
      AND resolved_at IS NULL AND resumed_at IS NULL)
    OR
    (status = 'accepted'
      AND (feedback IS NULL OR char_length(btrim(feedback)) BETWEEN 1 AND 4000)
      AND resolved_by_actor_id IS NOT NULL AND resolved_at IS NOT NULL
      AND resumed_at IS NOT NULL)
    OR
    (status = 'changes_requested' AND feedback IS NOT NULL
      AND char_length(btrim(feedback)) BETWEEN 1 AND 4000
      AND resolved_by_actor_id IS NOT NULL AND resolved_at IS NOT NULL
      AND resumed_at IS NOT NULL)
    OR
    (status = 'stale' AND feedback IS NULL AND resolved_by_actor_id IS NOT NULL
      AND resolved_at IS NOT NULL AND resumed_at IS NOT NULL)
  )
);

CREATE UNIQUE INDEX native_document_work_review_one_pending_per_work_idx
  ON native_document_work_review_dependencies(work_id)
  WHERE status = 'pending';

CREATE INDEX native_document_work_review_document_idx
  ON native_document_work_review_dependencies(document_id, created_at, review_id);

CREATE FUNCTION orgintel_guard_native_document_work_review_mutation()
RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'native document Work-review history is immutable';
  END IF;
  IF NEW.review_id <> OLD.review_id
    OR NEW.document_id <> OLD.document_id
    OR NEW.requested_version_id <> OLD.requested_version_id
    OR NEW.work_id <> OLD.work_id
    OR NEW.attempt_id <> OLD.attempt_id
    OR NEW.work_revision <> OLD.work_revision
    OR NEW.reviewer_actor_id <> OLD.reviewer_actor_id
    OR NEW.created_at <> OLD.created_at THEN
    RAISE EXCEPTION 'native document Work-review target is immutable';
  END IF;
  IF OLD.status <> 'pending'
    OR NEW.status NOT IN ('accepted', 'changes_requested', 'stale') THEN
    RAISE EXCEPTION 'invalid native document Work-review transition';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_work_review_target_is_immutable
  BEFORE UPDATE OR DELETE ON native_document_work_review_dependencies
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_work_review_mutation();

ALTER TABLE native_document_command_receipts
  DROP CONSTRAINT native_document_command_receipts_operation_check;

ALTER TABLE native_document_command_receipts
  ADD CONSTRAINT native_document_command_receipts_operation_check CHECK (operation IN (
    'document_create',
    'document_metadata_update',
    'markdown_import',
    'named_version_create',
    'named_version_restore',
    'participant_set',
    'participant_remove',
    'comment_thread_create',
    'comment_reply',
    'comment_thread_resolve',
    'review_request',
    'review_accept',
    'review_request_changes',
    'revision_propose',
    'revision_accept',
    'revision_reject'
  ));
