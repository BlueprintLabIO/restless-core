-- Sprint 49: durable Actor continuity is an immutable, versioned checkpoint
-- plus typed links back to authoritative company sources.  The checkpoint is
-- working memory, never a second Work/Room/Decision store and never Authority.

CREATE TYPE actor_checkpoint_session_kind AS ENUM (
  'work_attempt',
  'cognitive_session'
);

CREATE TYPE actor_context_epistemic_kind AS ENUM (
  'observation',
  'claim',
  'hypothesis',
  'assumption',
  'judgment',
  'principle',
  'decision',
  'unknown'
);

CREATE TYPE actor_context_source_kind AS ENUM (
  'actor',
  'work',
  'attempt',
  'room',
  'message',
  'decision',
  'artifact',
  'document',
  'runtime_file',
  'git_commit',
  'external'
);

CREATE TYPE actor_context_source_trust AS ENUM (
  'company_state',
  'company_decision',
  'authenticated_actor_input',
  'runtime_untrusted_evidence',
  'external_untrusted'
);

CREATE TABLE actor_context_checkpoints (
  id                    UUID PRIMARY KEY,
  actor_id              TEXT NOT NULL REFERENCES actors(id),
  checkpoint_version    BIGINT NOT NULL CHECK (checkpoint_version > 0),
  schema_version        SMALLINT NOT NULL CHECK (schema_version = 1),
  client_command_id     TEXT NOT NULL,
  client_payload_sha256 TEXT NOT NULL,
  session_kind          actor_checkpoint_session_kind NOT NULL,
  work_id               UUID REFERENCES work(id),
  attempt_id            UUID REFERENCES work_attempts(id),
  focused_mention_id    UUID REFERENCES message_mentions(id),
  body                  JSONB NOT NULL CHECK (
    jsonb_typeof(body) = 'object'
    AND octet_length(body::text) <= 32768
  ),
  recorded_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (actor_id, checkpoint_version),
  UNIQUE (actor_id, client_command_id),
  CHECK (length(btrim(client_command_id)) BETWEEN 1 AND 128),
  CHECK (client_payload_sha256 ~ '^[0-9a-f]{64}$'),
  CHECK (
    (session_kind = 'work_attempt'
      AND work_id IS NOT NULL AND attempt_id IS NOT NULL
      AND focused_mention_id IS NULL)
    OR
    (session_kind = 'cognitive_session'
      AND work_id IS NULL AND attempt_id IS NULL)
  )
);

CREATE INDEX actor_context_checkpoints_current_idx
  ON actor_context_checkpoints (actor_id, checkpoint_version DESC);
CREATE INDEX actor_context_checkpoints_work_idx
  ON actor_context_checkpoints (actor_id, work_id, checkpoint_version DESC)
  WHERE work_id IS NOT NULL;
CREATE INDEX actor_context_checkpoints_mention_idx
  ON actor_context_checkpoints (actor_id, focused_mention_id, checkpoint_version DESC)
  WHERE focused_mention_id IS NOT NULL;

CREATE TABLE actor_context_checkpoint_sources (
  checkpoint_id         UUID NOT NULL REFERENCES actor_context_checkpoints(id),
  ordinal               SMALLINT NOT NULL CHECK (ordinal BETWEEN 0 AND 63),
  epistemic_kind        actor_context_epistemic_kind NOT NULL,
  source_kind           actor_context_source_kind NOT NULL,
  source_trust          actor_context_source_trust NOT NULL,
  source_author_actor_id TEXT REFERENCES actors(id),
  source_ref            JSONB NOT NULL CHECK (
    jsonb_typeof(source_ref) = 'object'
    AND octet_length(source_ref::text) <= 8192
  ),
  statement             TEXT NOT NULL,
  scope                 TEXT,
  expires_at            TIMESTAMPTZ,
  observed_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (checkpoint_id, ordinal),
  CHECK (source_ref->>'kind' = source_kind::text),
  CHECK (length(btrim(statement)) BETWEEN 1 AND 4000),
  CHECK (scope IS NULL OR length(btrim(scope)) BETWEEN 1 AND 1000)
);

CREATE INDEX actor_context_checkpoint_sources_kind_idx
  ON actor_context_checkpoint_sources (source_kind, checkpoint_id);

-- Checkpoints and their provenance are historical receipts. A correction is
-- another optimistic version, never mutation of what a prior session said.
CREATE FUNCTION orgintel_reject_actor_context_checkpoint_mutation()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'Actor context checkpoints and source links are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER actor_context_checkpoints_are_immutable
  BEFORE UPDATE OR DELETE ON actor_context_checkpoints
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_actor_context_checkpoint_mutation();

CREATE TRIGGER actor_context_checkpoint_sources_are_immutable
  BEFORE UPDATE OR DELETE ON actor_context_checkpoint_sources
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_actor_context_checkpoint_mutation();
