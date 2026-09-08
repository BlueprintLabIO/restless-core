-- Runtime document import provenance must remain inspectable after the
-- compactable `events` stream has discarded its delivery hints. The imported
-- named version remains the authoritative body; this table records only the
-- bounded source/checksum receipt needed to explain and safely replay the
-- explicit Runtime boundary.

CREATE TABLE native_document_runtime_imports (
  document_id UUID NOT NULL,
  imported_version_id UUID NOT NULL,
  target_base_version_id UUID NOT NULL,
  envelope_schema TEXT NOT NULL CHECK (
    envelope_schema = 'restless.native-document.runtime-checkpoint'
  ),
  envelope_version SMALLINT NOT NULL CHECK (envelope_version = 1),
  envelope_checksum TEXT NOT NULL CHECK (
    envelope_checksum ~ '^[0-9a-f]{64}$'
  ),
  source_company_id UUID,
  source_company_key TEXT NOT NULL CHECK (
    char_length(btrim(source_company_key)) BETWEEN 1 AND 128
  ),
  source_document_id UUID NOT NULL,
  source_named_version_id UUID NOT NULL,
  source_named_version_number BIGINT NOT NULL CHECK (
    source_named_version_number >= 1
  ),
  source_content_schema_version SMALLINT NOT NULL CHECK (
    source_content_schema_version = 1
  ),
  source_document_status native_document_status NOT NULL,
  source_restored_from_version_id UUID,
  source_created_by_actor_id TEXT NOT NULL CHECK (
    char_length(btrim(source_created_by_actor_id)) BETWEEN 1 AND 200
  ),
  source_version_reason TEXT NOT NULL CHECK (
    char_length(btrim(source_version_reason)) BETWEEN 1 AND 500
  ),
  source_version_created_at TIMESTAMPTZ NOT NULL,
  source_content_hash TEXT NOT NULL CHECK (
    source_content_hash ~ '^[0-9a-f]{64}$'
  ),
  imported_content_hash TEXT NOT NULL CHECK (
    imported_content_hash ~ '^[0-9a-f]{64}$'
  ),
  imported_by_actor_id TEXT NOT NULL REFERENCES actors(id),
  imported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (document_id, imported_version_id),
  FOREIGN KEY (document_id, imported_version_id)
    REFERENCES native_document_versions(document_id, id),
  FOREIGN KEY (document_id, target_base_version_id)
    REFERENCES native_document_versions(document_id, id)
);

CREATE INDEX native_document_runtime_imports_source_idx
  ON native_document_runtime_imports (
    source_company_key,
    source_document_id,
    source_named_version_id
  );

-- An import receipt explains an immutable named version. Rewriting or deleting
-- that receipt would make the version's provenance unverifiable, so corrections
-- are represented by a later import rather than in-place mutation.
CREATE FUNCTION orgintel_reject_native_document_runtime_import_mutation()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'native document Runtime import receipts are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_document_runtime_imports_are_immutable
  BEFORE UPDATE OR DELETE ON native_document_runtime_imports
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_runtime_import_mutation();
