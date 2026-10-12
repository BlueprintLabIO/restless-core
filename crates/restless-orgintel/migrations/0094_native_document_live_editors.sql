-- Who actually changed a document's live body (S66), not only who could have. The collaboration
-- sidecar notes the people whose edits it stored; the next version names them as its editors.
-- A sidecar that notes nobody leaves the earlier rule in place: whoever held a write session.
CREATE TABLE native_document_live_editors (
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  edited_at TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (document_id, actor_id)
);

-- Only someone holding a write session on this document, in this company, can be noted.
CREATE FUNCTION orgintel_native_document_note_editors(
  requested_company_id UUID,
  requested_document_id UUID,
  requested_actor_ids TEXT[]
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
  noted INTEGER;
BEGIN
  IF requested_company_id IS NULL
     OR requested_document_id IS NULL
     OR requested_actor_ids IS NULL
     OR pg_catalog.cardinality(requested_actor_ids) > 64
  THEN
    RAISE EXCEPTION 'invalid live editor note' USING ERRCODE = '22023';
  END IF;
  INSERT INTO native_document_live_editors (document_id, actor_id, edited_at)
  SELECT DISTINCT requested_document_id, session.actor_id, pg_catalog.now()
  FROM native_document_collaboration_sessions session
  WHERE session.company_id = requested_company_id
    AND session.document_id = requested_document_id
    AND session.access = 'write'
    AND session.actor_id = ANY (requested_actor_ids)
    AND session.expires_at > pg_catalog.now() - INTERVAL '10 minutes'
  ON CONFLICT (document_id, actor_id) DO UPDATE SET edited_at = EXCLUDED.edited_at;
  GET DIAGNOSTICS noted = ROW_COUNT;
  RETURN noted;
END;
$$;

REVOKE ALL ON TABLE native_document_live_editors FROM PUBLIC;
REVOKE ALL ON FUNCTION orgintel_native_document_note_editors(UUID, UUID, TEXT[]) FROM PUBLIC;
