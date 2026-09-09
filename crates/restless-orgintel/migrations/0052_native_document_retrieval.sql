-- Native Document retrieval is a rebuildable projection of authoritative
-- metadata and the exact current named version. Search rows and typed
-- references may be deleted wholesale and reconstructed without changing a
-- Document or inventing a second body truth.

ALTER TABLE native_document_command_receipts
  DROP CONSTRAINT native_document_command_receipts_operation_check;

ALTER TABLE native_document_command_receipts
  ADD CONSTRAINT native_document_command_receipts_operation_check CHECK (operation IN (
    'document_create',
    'document_metadata_update',
    'named_version_create',
    'named_version_restore',
    'markdown_import',
    'participant_set',
    'participant_remove',
    'comment_thread_create',
    'comment_reply',
    'comment_thread_resolve',
    'review_request',
    'review_accept',
    'revision_propose',
    'revision_accept',
    'revision_reject'
  ));

CREATE TABLE native_document_search_projection (
  document_id UUID PRIMARY KEY REFERENCES native_documents(id) ON DELETE CASCADE,
  named_version_id UUID NOT NULL,
  title TEXT NOT NULL,
  kind native_document_kind NOT NULL,
  plain_text TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  search_vector TSVECTOR GENERATED ALWAYS AS (
    to_tsvector('simple', title || ' ' || plain_text)
  ) STORED,
  FOREIGN KEY (document_id, named_version_id)
    REFERENCES native_document_versions(document_id, id)
);

CREATE INDEX native_document_search_projection_text_idx
  ON native_document_search_projection USING GIN (search_vector);
CREATE INDEX native_document_search_projection_title_trgm_idx
  ON native_document_search_projection USING GIN (title public.gin_trgm_ops);

-- Only the editor-schema's explicit typed `reference` nodes enter this
-- projection. Ordinary hyperlinks remain ordinary hyperlinks; no URL guessing
-- creates organisational meaning.
CREATE TABLE native_document_reference_projection (
  source_document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  source_named_version_id UUID NOT NULL,
  ordinal BIGINT NOT NULL CHECK (ordinal >= 1),
  target_kind TEXT NOT NULL CHECK (
    target_kind IN ('work', 'decision', 'attention', 'document', 'artifact', 'actor')
  ),
  target_id TEXT NOT NULL CHECK (char_length(target_id) BETWEEN 1 AND 200),
  label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 300),
  PRIMARY KEY (source_document_id, ordinal),
  FOREIGN KEY (source_document_id, source_named_version_id)
    REFERENCES native_document_versions(document_id, id)
);

CREATE INDEX native_document_reference_projection_target_idx
  ON native_document_reference_projection (
    target_kind,
    target_id,
    source_document_id
  );

CREATE FUNCTION orgintel_refresh_native_document_retrieval(target_document_id UUID)
RETURNS VOID AS $$
BEGIN
  DELETE FROM native_document_reference_projection
  WHERE source_document_id=target_document_id;

  INSERT INTO native_document_search_projection (
    document_id,
    named_version_id,
    title,
    kind,
    plain_text,
    updated_at
  )
  SELECT document.id,
         document.current_named_version_id,
         document.title,
         document.kind,
         version.plain_text,
         document.updated_at
  FROM native_documents document
  JOIN native_document_versions version
    ON version.document_id=document.id
   AND version.id=document.current_named_version_id
  WHERE document.id=target_document_id
  ON CONFLICT (document_id) DO UPDATE SET
    named_version_id=EXCLUDED.named_version_id,
    title=EXCLUDED.title,
    kind=EXCLUDED.kind,
    plain_text=EXCLUDED.plain_text,
    updated_at=EXCLUDED.updated_at;

  INSERT INTO native_document_reference_projection (
    source_document_id,
    source_named_version_id,
    ordinal,
    target_kind,
    target_id,
    label
  )
  SELECT document.id,
         document.current_named_version_id,
         reference.ordinal,
         reference.node->'attrs'->>'kind',
         reference.node->'attrs'->>'id',
         reference.node->'attrs'->>'label'
  FROM native_documents document
  JOIN native_document_versions version
    ON version.document_id=document.id
   AND version.id=document.current_named_version_id
  CROSS JOIN LATERAL jsonb_path_query(
    version.content_json,
    'strict $.** ? (@.type == "reference")'
  ) WITH ORDINALITY AS reference(node, ordinal)
  WHERE document.id=target_document_id
    AND reference.node->'attrs'->>'kind' IN (
      'work', 'decision', 'attention', 'document', 'artifact', 'actor'
    )
    AND char_length(reference.node->'attrs'->>'id') BETWEEN 1 AND 200
    AND char_length(reference.node->'attrs'->>'label') BETWEEN 1 AND 300;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION orgintel_native_document_metadata_retrieval_trigger()
RETURNS TRIGGER AS $$
BEGIN
  PERFORM orgintel_refresh_native_document_retrieval(NEW.id);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_metadata_refreshes_retrieval
  AFTER INSERT OR UPDATE OF title,kind,current_named_version_id,updated_at
  ON native_documents
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_metadata_retrieval_trigger();

-- Creation inserts the metadata row before its deferred first-version row.
-- This second trigger closes that intentional gap; later named versions are
-- projected only after the current-version pointer advances.
CREATE FUNCTION orgintel_native_document_version_retrieval_trigger()
RETURNS TRIGGER AS $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM native_documents
    WHERE id=NEW.document_id AND current_named_version_id=NEW.id
  ) THEN
    PERFORM orgintel_refresh_native_document_retrieval(NEW.document_id);
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_version_refreshes_retrieval
  AFTER INSERT ON native_document_versions
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_version_retrieval_trigger();

-- The rebuild is deliberately database-local and derives every byte from
-- authoritative rows. Operations code may run it after detecting projection
-- loss without needing a Runtime, event history, or retained export.
CREATE FUNCTION orgintel_rebuild_native_document_retrieval()
RETURNS BIGINT AS $$
DECLARE
  document UUID;
  rebuilt BIGINT := 0;
BEGIN
  DELETE FROM native_document_reference_projection;
  DELETE FROM native_document_search_projection;
  FOR document IN SELECT id FROM native_documents ORDER BY id LOOP
    PERFORM orgintel_refresh_native_document_retrieval(document);
    rebuilt := rebuilt + 1;
  END LOOP;
  RETURN rebuilt;
END;
$$ LANGUAGE plpgsql;

SELECT orgintel_rebuild_native_document_retrieval();
