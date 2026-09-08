-- Native Docs phase 1: company-scoped metadata, explicit Actor access and
-- immutable named structured-content versions. The live collaborative Yjs
-- body arrives in a later migration; these rows remain durable checkpoints,
-- never a shadow file or a second concurrent editor truth.

CREATE TYPE native_document_kind AS ENUM (
  'brief',
  'plan',
  'decision_note',
  'report',
  'review',
  'handbook',
  'operating_note',
  'freeform'
);

CREATE TYPE native_document_status AS ENUM (
  'draft',
  'in_review',
  'accepted',
  'archived'
);

CREATE TYPE native_document_visibility AS ENUM ('company', 'participants');
CREATE TYPE native_document_access AS ENUM ('read', 'comment', 'edit');

CREATE TABLE native_documents (
  id UUID PRIMARY KEY,
  title TEXT NOT NULL CHECK (
    char_length(btrim(title)) BETWEEN 1 AND 200
  ),
  kind native_document_kind NOT NULL,
  status native_document_status NOT NULL DEFAULT 'draft',
  visibility native_document_visibility NOT NULL DEFAULT 'company',
  linked_room_id UUID REFERENCES rooms(id),
  -- A participant-visible Doc may dynamically inherit an active Room's
  -- audience. Turning this off is the explicit "narrow to Doc participants"
  -- operation; membership is never copied into a second ACL.
  inherit_room_visibility BOOLEAN NOT NULL DEFAULT FALSE,
  owner_actor_id TEXT NOT NULL REFERENCES actors(id),
  current_named_version_id UUID NOT NULL,
  created_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  version BIGINT NOT NULL DEFAULT 1 CHECK (version >= 1),
  CHECK (
    NOT inherit_room_visibility
    OR (linked_room_id IS NOT NULL AND visibility = 'participants')
  )
);

CREATE TABLE native_document_versions (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL REFERENCES native_documents(id),
  version_number BIGINT NOT NULL CHECK (version_number >= 1),
  schema_version SMALLINT NOT NULL DEFAULT 1 CHECK (schema_version = 1),
  content_json JSONB NOT NULL CHECK (jsonb_typeof(content_json) = 'object'),
  plain_text TEXT NOT NULL,
  content_hash TEXT NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
  document_status native_document_status NOT NULL,
  restored_from_version_id UUID,
  created_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  reason TEXT NOT NULL CHECK (
    char_length(btrim(reason)) BETWEEN 1 AND 500
  ),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (document_id, version_number),
  UNIQUE (document_id, id)
);

ALTER TABLE native_document_versions
  ADD CONSTRAINT native_document_versions_restore_source_belongs_to_document
  FOREIGN KEY (document_id, restored_from_version_id)
  REFERENCES native_document_versions(document_id, id)
  DEFERRABLE INITIALLY DEFERRED;

-- The composite key prevents a document from pointing at another document's
-- version. Both sides are deferred so creation can atomically insert the
-- document and its first named version without a nullable "body not ready"
-- state becoming observable.
ALTER TABLE native_documents
  ADD CONSTRAINT native_documents_current_version_belongs_to_document
  FOREIGN KEY (id, current_named_version_id)
  REFERENCES native_document_versions(document_id, id)
  DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE native_document_participants (
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  access native_document_access NOT NULL,
  added_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  removed_at TIMESTAMPTZ,
  PRIMARY KEY (document_id, actor_id),
  CHECK (removed_at IS NULL OR removed_at >= added_at)
);

CREATE INDEX native_documents_updated_idx
  ON native_documents (updated_at DESC, id);
CREATE INDEX native_documents_active_idx
  ON native_documents (updated_at DESC, id)
  WHERE status <> 'archived';
CREATE INDEX native_documents_room_idx
  ON native_documents (linked_room_id, updated_at DESC)
  WHERE linked_room_id IS NOT NULL;
CREATE INDEX native_document_versions_document_idx
  ON native_document_versions (document_id, version_number DESC);
CREATE INDEX native_document_participants_actor_idx
  ON native_document_participants (actor_id, document_id)
  WHERE removed_at IS NULL;

-- Native Docs publish bounded refetch hints through the existing compactable
-- operational stream. `events.id` remains the one reconnect cursor; document
-- bodies, titles and version reasons remain in their authoritative tables and
-- never enter this stream.
ALTER TABLE events
  ADD COLUMN document_id UUID REFERENCES native_documents(id) ON DELETE CASCADE,
  ADD CONSTRAINT events_document_shape CHECK (
    document_id IS NULL
    OR (kind LIKE 'document.%.v1' AND jsonb_typeof(body) = 'object')
  );
CREATE INDEX events_document_cursor
  ON events (document_id, id) WHERE document_id IS NOT NULL;

CREATE TRIGGER native_documents_touch_updated_at
  BEFORE UPDATE ON native_documents
  FOR EACH ROW EXECUTE FUNCTION orgintel_touch_updated_at();

-- A named version is evidence, not a mutable cache. Restoring a checkpoint
-- inserts a new row and advances the current pointer; it never rewrites or
-- deletes the checkpoint being restored.
CREATE FUNCTION orgintel_reject_native_document_version_mutation()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'native document versions are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_versions_are_immutable
  BEFORE UPDATE OR DELETE ON native_document_versions
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_version_mutation();
