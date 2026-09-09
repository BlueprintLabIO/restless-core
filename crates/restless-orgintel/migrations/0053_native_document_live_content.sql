-- Native Docs realtime body state. The full merged Yjs binary is the only
-- mutable live body. Immutable named versions remain explicit checkpoints and
-- the JSON beside the binary is a readable projection, never a second writer.

-- pgcrypto is database-scoped while OrgIntel migrations are company-schema
-- scoped. Keep the extension in public and qualify its only use below.
DO $$
DECLARE
  extension_schema TEXT;
BEGIN
  SELECT namespace.nspname INTO extension_schema
  FROM pg_catalog.pg_extension extension
  JOIN pg_catalog.pg_namespace namespace ON namespace.oid=extension.extnamespace
  WHERE extension.extname='pgcrypto';

  IF extension_schema IS NULL THEN
    EXECUTE 'CREATE EXTENSION pgcrypto WITH SCHEMA public';
  ELSIF extension_schema <> 'public' THEN
    EXECUTE 'ALTER EXTENSION pgcrypto SET SCHEMA public';
  END IF;
END;
$$;

CREATE TABLE native_document_yjs_state (
  document_id UUID PRIMARY KEY REFERENCES native_documents(id) ON DELETE CASCADE,
  company_id UUID NOT NULL REFERENCES company_access_identity(company_id),
  format_version SMALLINT NOT NULL DEFAULT 1 CHECK (format_version = 1),
  yjs_state BYTEA NOT NULL CHECK (
    octet_length(yjs_state) BETWEEN 2 AND 8388608
  ),
  projection_json JSONB NOT NULL CHECK (
    jsonb_typeof(projection_json)='object'
    AND pg_column_size(projection_json) <= 2097152
  ),
  state_revision BIGINT NOT NULL CHECK (state_revision >= 1),
  content_hash TEXT GENERATED ALWAYS AS (
    pg_catalog.encode(public.digest(yjs_state, 'sha256'), 'hex')
  ) STORED,
  last_store_id UUID NOT NULL UNIQUE,
  seeded_from_named_version_id UUID NOT NULL,
  checkpoint_named_version_id UUID NOT NULL,
  checkpoint_state_revision BIGINT NOT NULL CHECK (
    checkpoint_state_revision >= 0
    AND checkpoint_state_revision <= state_revision
  ),
  persisted_at TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.now(),
  FOREIGN KEY (document_id, seeded_from_named_version_id)
    REFERENCES native_document_versions(document_id, id),
  FOREIGN KEY (document_id, checkpoint_named_version_id)
    REFERENCES native_document_versions(document_id, id)
);

CREATE INDEX native_document_yjs_state_company_idx
  ON native_document_yjs_state (company_id, persisted_at DESC);

-- Collaboration grants are short-lived and single use. Keeping consumed jtis
-- here makes replay refusal survive a sidecar restart. The sidecar login never
-- receives table access; it can only call the bounded functions below.
CREATE TABLE native_document_collaboration_sessions (
  session_id UUID PRIMARY KEY,
  company_id UUID NOT NULL REFERENCES company_access_identity(company_id),
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  access TEXT NOT NULL CHECK (access IN ('read', 'write')),
  issued_at TIMESTAMPTZ NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  consumed_at TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.now(),
  CHECK (expires_at > issued_at),
  CHECK (expires_at >= issued_at + INTERVAL '15 seconds'),
  CHECK (expires_at <= issued_at + INTERVAL '120 seconds')
);

CREATE INDEX native_document_collaboration_sessions_expiry_idx
  ON native_document_collaboration_sessions (expires_at);

-- Re-authorise the immutable company/document/Actor coordinates before
-- consuming a grant. A duplicate jti returns false without revealing whether
-- another session, company or document consumed it.
CREATE FUNCTION orgintel_native_document_collaboration_consume(
  requested_company_id UUID,
  requested_document_id UUID,
  requested_actor_id TEXT,
  requested_session_id UUID,
  requested_access TEXT,
  requested_issued_at TIMESTAMPTZ,
  requested_expires_at TIMESTAMPTZ
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
  identity_matches BOOLEAN;
  subject_matches BOOLEAN;
BEGIN
  IF requested_company_id='00000000-0000-0000-0000-000000000000'::UUID
     OR requested_document_id='00000000-0000-0000-0000-000000000000'::UUID
     OR requested_session_id='00000000-0000-0000-0000-000000000000'::UUID
     OR requested_access NOT IN ('read', 'write')
     OR requested_issued_at > pg_catalog.now() + INTERVAL '5 seconds'
     OR requested_expires_at <= pg_catalog.now()
     OR requested_expires_at < requested_issued_at + INTERVAL '15 seconds'
     OR requested_expires_at > requested_issued_at + INTERVAL '120 seconds'
  THEN
    RETURN FALSE;
  END IF;

  SELECT pg_catalog.count(*) = 1 INTO identity_matches
  FROM company_access_identity
  WHERE company_id=requested_company_id;

  SELECT pg_catalog.count(*) = 1 INTO subject_matches
  FROM native_documents document
  JOIN actors actor ON actor.id=requested_actor_id
  WHERE document.id=requested_document_id
    AND actor.retired_at IS NULL;

  IF NOT identity_matches OR NOT subject_matches THEN
    RETURN FALSE;
  END IF;

  DELETE FROM native_document_collaboration_sessions
  WHERE expires_at < pg_catalog.now() - INTERVAL '5 minutes';

  INSERT INTO native_document_collaboration_sessions (
    session_id,
    company_id,
    document_id,
    actor_id,
    access,
    issued_at,
    expires_at
  ) VALUES (
    requested_session_id,
    requested_company_id,
    requested_document_id,
    requested_actor_id,
    requested_access,
    requested_issued_at,
    requested_expires_at
  )
  ON CONFLICT (session_id) DO NOTHING;

  RETURN FOUND;
END;
$$;

-- Return either the exact durable live snapshot or the current immutable
-- named version from which the sidecar must seed its first Y.Doc. A drifted
-- checkpoint fails closed rather than silently choosing one of two writers.
CREATE FUNCTION orgintel_native_document_yjs_load(
  requested_company_id UUID,
  requested_document_id UUID
)
RETURNS TABLE (
  source_kind TEXT,
  state_revision BIGINT,
  yjs_state BYTEA,
  projection_json JSONB,
  seeded_from_named_version_id UUID,
  checkpoint_named_version_id UUID,
  checkpoint_state_revision BIGINT,
  content_hash TEXT,
  persisted_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
  current_version_id UUID;
  identity_matches BOOLEAN;
BEGIN
  SELECT pg_catalog.count(*) = 1 INTO identity_matches
  FROM company_access_identity
  WHERE company_id=requested_company_id;

  IF NOT identity_matches THEN
    RAISE EXCEPTION USING ERRCODE='P0002', MESSAGE='native document unavailable';
  END IF;

  SELECT document.current_named_version_id INTO current_version_id
  FROM native_documents document
  WHERE document.id=requested_document_id
  FOR SHARE;

  IF NOT FOUND THEN
    RAISE EXCEPTION USING ERRCODE='P0002', MESSAGE='native document unavailable';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM native_document_yjs_state live
    WHERE live.document_id=requested_document_id
      AND live.checkpoint_named_version_id<>current_version_id
  ) THEN
    RAISE EXCEPTION USING ERRCODE='40001', MESSAGE='native document checkpoint drift';
  END IF;

  RETURN QUERY
  SELECT 'state'::TEXT,
         live.state_revision,
         live.yjs_state,
         live.projection_json,
         live.seeded_from_named_version_id,
         live.checkpoint_named_version_id,
         live.checkpoint_state_revision,
         live.content_hash,
         live.persisted_at
  FROM native_document_yjs_state live
  WHERE live.document_id=requested_document_id;

  IF FOUND THEN
    RETURN;
  END IF;

  RETURN QUERY
  SELECT 'seed'::TEXT,
         0::BIGINT,
         NULL::BYTEA,
         version.content_json,
         version.id,
         version.id,
         0::BIGINT,
         version.content_hash,
         version.created_at
  FROM native_document_versions version
  WHERE version.document_id=requested_document_id
    AND version.id=current_version_id;

  IF NOT FOUND THEN
    RAISE EXCEPTION USING ERRCODE='P0002', MESSAGE='native document unavailable';
  END IF;
END;
$$;

-- Store one complete merged state with compare-and-swap semantics. Retrying a
-- lost response with the same store_id replays the original acknowledgement.
-- A stale writer receives the current bytes so it can merge and retry without
-- replacing another collaborator's update.
CREATE FUNCTION orgintel_native_document_yjs_store(
  requested_company_id UUID,
  requested_document_id UUID,
  expected_state_revision BIGINT,
  expected_checkpoint_named_version_id UUID,
  requested_store_id UUID,
  requested_yjs_state BYTEA,
  requested_projection_json JSONB
)
RETURNS TABLE (
  outcome TEXT,
  state_revision BIGINT,
  checkpoint_named_version_id UUID,
  content_hash TEXT,
  persisted_at TIMESTAMPTZ,
  current_yjs_state BYTEA,
  current_projection_json JSONB
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
  current_version_id UUID;
  identity_matches BOOLEAN;
  live native_document_yjs_state%ROWTYPE;
BEGIN
  IF expected_state_revision < 0
     OR requested_yjs_state IS NULL
     OR pg_catalog.octet_length(requested_yjs_state) NOT BETWEEN 2 AND 8388608
     OR requested_projection_json IS NULL
     OR pg_catalog.jsonb_typeof(requested_projection_json)<>'object'
     OR pg_catalog.pg_column_size(requested_projection_json) > 2097152
  THEN
    RAISE EXCEPTION USING ERRCODE='22023', MESSAGE='native document live content is invalid';
  END IF;

  SELECT pg_catalog.count(*) = 1 INTO identity_matches
  FROM company_access_identity
  WHERE company_id=requested_company_id;

  IF NOT identity_matches THEN
    RAISE EXCEPTION USING ERRCODE='P0002', MESSAGE='native document unavailable';
  END IF;

  SELECT document.current_named_version_id INTO current_version_id
  FROM native_documents document
  WHERE document.id=requested_document_id
  FOR SHARE;

  IF NOT FOUND THEN
    RAISE EXCEPTION USING ERRCODE='P0002', MESSAGE='native document unavailable';
  END IF;

  SELECT * INTO live
  FROM native_document_yjs_state current_live
  WHERE current_live.document_id=requested_document_id
  FOR UPDATE;

  IF NOT FOUND THEN
    IF expected_state_revision<>0
       OR expected_checkpoint_named_version_id<>current_version_id
    THEN
      RETURN QUERY SELECT
        'conflict'::TEXT,
        0::BIGINT,
        current_version_id,
        version.content_hash,
        version.created_at,
        NULL::BYTEA,
        version.content_json
      FROM native_document_versions version
      WHERE version.document_id=requested_document_id
        AND version.id=current_version_id;
      RETURN;
    END IF;

    INSERT INTO native_document_yjs_state (
      document_id,
      company_id,
      yjs_state,
      projection_json,
      state_revision,
      last_store_id,
      seeded_from_named_version_id,
      checkpoint_named_version_id,
      checkpoint_state_revision
    ) VALUES (
      requested_document_id,
      requested_company_id,
      requested_yjs_state,
      requested_projection_json,
      1,
      requested_store_id,
      current_version_id,
      current_version_id,
      0
    )
    ON CONFLICT (document_id) DO NOTHING
    RETURNING * INTO live;

    IF FOUND THEN
      RETURN QUERY SELECT
        'stored'::TEXT,
        live.state_revision,
        live.checkpoint_named_version_id,
        live.content_hash,
        live.persisted_at,
        NULL::BYTEA,
        NULL::JSONB;
      RETURN;
    END IF;

    -- Another initial writer committed while both held a shared document
    -- lock. Observe that winner and continue through replay/conflict handling.
    SELECT * INTO live
    FROM native_document_yjs_state current_live
    WHERE current_live.document_id=requested_document_id
    FOR UPDATE;

    IF NOT FOUND THEN
      RAISE EXCEPTION USING ERRCODE='40001', MESSAGE='native document live state changed';
    END IF;
  END IF;

  IF live.last_store_id=requested_store_id THEN
    IF live.state_revision<>expected_state_revision + 1
       OR live.checkpoint_named_version_id<>expected_checkpoint_named_version_id
       OR live.yjs_state<>requested_yjs_state
       OR live.projection_json<>requested_projection_json
    THEN
      RAISE EXCEPTION USING ERRCODE='23505', MESSAGE='native document store id collision';
    END IF;

    RETURN QUERY SELECT
      'replayed'::TEXT,
      live.state_revision,
      live.checkpoint_named_version_id,
      live.content_hash,
      live.persisted_at,
      NULL::BYTEA,
      NULL::JSONB;
    RETURN;
  END IF;

  IF live.state_revision<>expected_state_revision
     OR live.checkpoint_named_version_id<>expected_checkpoint_named_version_id
     OR current_version_id<>live.checkpoint_named_version_id
  THEN
    RETURN QUERY SELECT
      'conflict'::TEXT,
      live.state_revision,
      live.checkpoint_named_version_id,
      live.content_hash,
      live.persisted_at,
      live.yjs_state,
      live.projection_json;
    RETURN;
  END IF;

  UPDATE native_document_yjs_state current_live
  SET yjs_state=requested_yjs_state,
      projection_json=requested_projection_json,
      state_revision=current_live.state_revision + 1,
      last_store_id=requested_store_id,
      persisted_at=pg_catalog.now()
  WHERE current_live.document_id=requested_document_id
  RETURNING * INTO live;

  RETURN QUERY SELECT
    'stored'::TEXT,
    live.state_revision,
    live.checkpoint_named_version_id,
    live.content_hash,
    live.persisted_at,
    NULL::BYTEA,
    NULL::JSONB;
END;
$$;

REVOKE ALL ON TABLE native_document_yjs_state FROM PUBLIC;
REVOKE ALL ON TABLE native_document_collaboration_sessions FROM PUBLIC;
REVOKE ALL ON FUNCTION orgintel_native_document_collaboration_consume(
  UUID, UUID, TEXT, UUID, TEXT, TIMESTAMPTZ, TIMESTAMPTZ
) FROM PUBLIC;
REVOKE ALL ON FUNCTION orgintel_native_document_yjs_load(UUID, UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION orgintel_native_document_yjs_store(
  UUID, UUID, BIGINT, UUID, UUID, BYTEA, JSONB
) FROM PUBLIC;
