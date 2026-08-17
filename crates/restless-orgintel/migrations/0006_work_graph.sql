-- S05-T8: the repeated Aris handoff failures earned a real Work graph.
--
-- This MIGRATES the existing commitment primitive; it does not create a
-- parallel task/workflow truth. Work remains mutable, repairable OrgIntel
-- state. Files and Git remain the artifacts, and Authority remains the sole
-- owner of consequential effects.

DROP TRIGGER IF EXISTS commitment_completed_notify ON commitments;
DROP TRIGGER IF EXISTS commitments_touch_updated_at ON commitments;

ALTER TYPE commitment_state RENAME TO work_state;
ALTER TABLE commitments RENAME TO work;
ALTER TABLE work RENAME COLUMN body TO outcome;
ALTER TABLE work RENAME COLUMN state TO status;

ALTER TABLE work
  ADD COLUMN priority SMALLINT NOT NULL DEFAULT 0,
  ADD COLUMN expected_artifact TEXT NOT NULL DEFAULT '',
  ADD COLUMN repo TEXT,
  ADD COLUMN base_ref TEXT,
  ADD COLUMN integration_branch TEXT,
  ADD COLUMN worktree TEXT,
  ADD COLUMN revision BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0),
  ADD COLUMN attempt_limit INTEGER CHECK (attempt_limit IS NULL OR attempt_limit > 0);

CREATE TYPE work_edge_kind AS ENUM ('requires', 'revises');

CREATE TABLE work_edges (
    from_work_id UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    to_work_id   UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    kind         work_edge_kind NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (from_work_id, to_work_id, kind),
    CHECK (from_work_id <> to_work_id)
);
CREATE INDEX work_edges_to ON work_edges (to_work_id, kind);

CREATE TYPE work_attempt_state AS ENUM (
    'running',
    'produced',
    'changes_requested',
    'blocked',
    'failed',
    'abandoned',
    'superseded'
);

CREATE TABLE work_attempts (
    id                UUID PRIMARY KEY,
    work_id           UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    revision          BIGINT NOT NULL CHECK (revision > 0),
    attempt_no        INTEGER NOT NULL CHECK (attempt_no > 0),
    actor_id          TEXT NOT NULL REFERENCES actors(id),
    session_id        TEXT NOT NULL UNIQUE,
    state             work_attempt_state NOT NULL DEFAULT 'running',
    trigger           TEXT NOT NULL,
    input_fingerprint TEXT NOT NULL,
    feedback_cursor   BIGINT NOT NULL DEFAULT 0,
    model             TEXT,
    started_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at       TIMESTAMPTZ,
    summary           TEXT NOT NULL DEFAULT '',
    UNIQUE (work_id, revision, attempt_no)
);
CREATE UNIQUE INDEX work_one_running_attempt
  ON work_attempts (work_id) WHERE state = 'running';

CREATE TYPE artifact_ref_state AS ENUM (
    'available', 'stale', 'missing', 'superseded', 'unknown'
);

ALTER TABLE artifact_refs
  ADD COLUMN work_id UUID REFERENCES work(id) ON DELETE SET NULL,
  ADD COLUMN attempt_id UUID REFERENCES work_attempts(id) ON DELETE SET NULL,
  ADD COLUMN digest TEXT,
  ADD COLUMN source_commit TEXT,
  ADD COLUMN runtime_generation TEXT,
  ADD COLUMN label TEXT NOT NULL DEFAULT '',
  ADD COLUMN state artifact_ref_state NOT NULL DEFAULT 'available',
  ADD COLUMN superseded_at TIMESTAMPTZ;
CREATE INDEX artifact_refs_work ON artifact_refs (work_id, created_at);
CREATE INDEX artifact_refs_attempt ON artifact_refs (attempt_id);

CREATE TABLE work_attempt_inputs (
    attempt_id     UUID NOT NULL REFERENCES work_attempts(id) ON DELETE CASCADE,
    artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
    PRIMARY KEY (attempt_id, artifact_ref_id)
);

-- Conversation remains ordinary messages. These two small link tables only
-- say which free-form messages affect a Work revision and which exact Attempt
-- consumed them.
CREATE TABLE work_feedback (
    work_id      UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    message_id   BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    linked_by    TEXT NOT NULL REFERENCES actors(id),
    linked_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (work_id, message_id)
);

CREATE TABLE work_attempt_feedback (
    attempt_id   UUID NOT NULL REFERENCES work_attempts(id) ON DELETE CASCADE,
    message_id   BIGINT NOT NULL REFERENCES messages(id),
    PRIMARY KEY (attempt_id, message_id)
);

CREATE TABLE work_gates (
    id          UUID PRIMARY KEY,
    work_id     UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    cwd         TEXT NOT NULL,
    command     JSONB NOT NULL CHECK (jsonb_typeof(command) = 'array'),
    created_by  TEXT NOT NULL REFERENCES actors(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (work_id, name)
);

CREATE TABLE work_gate_runs (
    id            UUID PRIMARY KEY,
    gate_id       UUID NOT NULL REFERENCES work_gates(id) ON DELETE CASCADE,
    attempt_id    UUID NOT NULL REFERENCES work_attempts(id) ON DELETE CASCADE,
    exit_code     INTEGER,
    output_digest TEXT NOT NULL,
    output_excerpt TEXT NOT NULL,
    passed        BOOLEAN NOT NULL,
    ran_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (gate_id, attempt_id)
);

CREATE TYPE owner_handoff_category AS ENUM (
    'identity',
    'captcha',
    'mfa',
    'legal_attestation',
    'payment_confirmation',
    'owner_judgement'
);
CREATE TYPE owner_handoff_state AS ENUM (
    'pending', 'resolved', 'declined', 'withdrawn'
);

CREATE TABLE owner_handoffs (
    id                UUID PRIMARY KEY,
    work_id           UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    attempt_id        UUID REFERENCES work_attempts(id) ON DELETE SET NULL,
    requested_by      TEXT NOT NULL REFERENCES actors(id),
    category          owner_handoff_category NOT NULL,
    requested_action  TEXT NOT NULL,
    prepared_state    TEXT NOT NULL,
    resume_condition  TEXT NOT NULL,
    state             owner_handoff_state NOT NULL DEFAULT 'pending',
    resolution        TEXT NOT NULL DEFAULT '',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at       TIMESTAMPTZ
);
CREATE UNIQUE INDEX one_pending_handoff_per_work
  ON owner_handoffs (work_id) WHERE state = 'pending';

-- Time is a legitimate dependency, but it is not hidden inside an event row.
-- Most Work moves on graph events; this table is only for an explicit time
-- condition with a reason.
CREATE TABLE schedules (
    id          UUID PRIMARY KEY,
    actor_id    TEXT NOT NULL REFERENCES actors(id),
    work_id     UUID REFERENCES work(id) ON DELETE CASCADE,
    reason      TEXT NOT NULL,
    fire_at     TIMESTAMPTZ NOT NULL,
    fired_at    TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX schedules_due ON schedules (fire_at)
  WHERE fired_at IS NULL AND cancelled_at IS NULL;

CREATE OR REPLACE FUNCTION orgintel_touch_updated_at() RETURNS trigger AS $$
BEGIN
  NEW.updated_at := now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER work_touch_updated_at
  BEFORE UPDATE ON work
  FOR EACH ROW EXECUTE FUNCTION orgintel_touch_updated_at();

-- The scheduler wakes on facts landing, not on whichever API happened to
-- write them. Payloads carry identifiers only; the scheduler rereads the
-- canonical rows before acting.
CREATE OR REPLACE FUNCTION orgintel_notify() RETURNS trigger AS $$
BEGIN
  IF TG_TABLE_NAME = 'work' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', TG_TABLE_SCHEMA,
      'kind', 'work_changed',
      'body', json_build_object(
        'work_id', NEW.id,
        'owner', NEW.owner_id,
        'status', NEW.status,
        'revision', NEW.revision
      )
    )::text);
  ELSIF TG_TABLE_NAME = 'artifact_refs' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', TG_TABLE_SCHEMA,
      'kind', 'artifact_linked',
      'body', json_build_object(
        'artifact_ref_id', NEW.id,
        'work_id', NEW.work_id,
        'attempt_id', NEW.attempt_id
      )
    )::text);
  ELSIF TG_TABLE_NAME = 'owner_handoffs' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', TG_TABLE_SCHEMA,
      'kind', 'handoff_changed',
      'body', json_build_object(
        'handoff_id', NEW.id,
        'work_id', NEW.work_id,
        'state', NEW.state
      )
    )::text);
  ELSIF TG_TABLE_NAME = 'messages' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', TG_TABLE_SCHEMA,
      'kind', 'message',
      'body', json_build_object(
        'message_id', NEW.id,
        'to', NEW.to_actor,
        'from', NEW.from_actor
      )
    )::text);
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER work_changed_notify
  AFTER INSERT OR UPDATE OF status, revision, owner_id ON work
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify();

CREATE TRIGGER artifact_linked_notify
  AFTER INSERT ON artifact_refs
  FOR EACH ROW WHEN (NEW.work_id IS NOT NULL)
  EXECUTE FUNCTION orgintel_notify();

CREATE TRIGGER handoff_changed_notify
  AFTER INSERT OR UPDATE OF state ON owner_handoffs
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify();
