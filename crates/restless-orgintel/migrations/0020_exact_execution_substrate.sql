-- Sprint 26: exact, hermetic, event-driven Attempt execution.
--
-- Work remains the durable responsibility and Git/Runtime retain file custody.
-- These columns make the already-existing Attempt/gate/promotion boundary
-- checkable and recoverable instead of leaving coordinates in prose.

ALTER TABLE work_attempts
  ADD COLUMN requested_source_ref TEXT,
  ADD COLUMN source_commit TEXT,
  ADD COLUMN source_tree TEXT,
  ADD COLUMN gate_set_digest TEXT NOT NULL DEFAULT '',
  ADD COLUMN environment_fingerprint TEXT NOT NULL DEFAULT '',
  ADD COLUMN materialized_at TIMESTAMPTZ,
  ADD COLUMN interrupt_requested_at TIMESTAMPTZ,
  ADD COLUMN interrupt_requested_by TEXT REFERENCES actors(id),
  ADD COLUMN interrupt_reason TEXT,
  ADD COLUMN feedback_checkpoint_cursor BIGINT NOT NULL DEFAULT 0;

ALTER TABLE work_gates
  ADD COLUMN stage TEXT NOT NULL DEFAULT 'cumulative'
    CHECK (stage IN ('focused', 'blind', 'cumulative')),
  ADD COLUMN timeout_seconds INTEGER NOT NULL DEFAULT 900
    CHECK (timeout_seconds BETWEEN 1 AND 7200),
  ADD COLUMN resources JSONB NOT NULL DEFAULT '[]'::jsonb
    CHECK (jsonb_typeof(resources) = 'array');

ALTER TABLE work_gate_runs
  ADD COLUMN candidate_tree TEXT NOT NULL DEFAULT '',
  ADD COLUMN definition_digest TEXT NOT NULL DEFAULT '',
  ADD COLUMN toolchain_fingerprint TEXT NOT NULL DEFAULT '',
  ADD COLUMN status TEXT NOT NULL DEFAULT 'conclusive'
    CHECK (status IN ('conclusive', 'cached', 'timeout', 'infrastructure_error', 'cancelled')),
  ADD COLUMN duration_ms BIGINT,
  ADD COLUMN cache_source_run_id UUID REFERENCES work_gate_runs(id),
  ADD COLUMN leaked_processes INTEGER NOT NULL DEFAULT 0 CHECK (leaked_processes >= 0);

CREATE INDEX work_gate_runs_exact_evidence
  ON work_gate_runs (gate_id, candidate_tree, definition_digest, toolchain_fingerprint, ran_at)
  WHERE status IN ('conclusive', 'cached');

CREATE TABLE runtime_resource_leases (
    id          UUID PRIMARY KEY,
    attempt_id  UUID NOT NULL REFERENCES work_attempts(id) ON DELETE CASCADE,
    gate_id     UUID REFERENCES work_gates(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('port', 'display', 'tempdir', 'process_group')),
    value       TEXT NOT NULL,
    holder_token TEXT NOT NULL,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    released_at TIMESTAMPTZ,
    release_reason TEXT
);
CREATE UNIQUE INDEX runtime_resource_one_live_value
  ON runtime_resource_leases (kind, value) WHERE released_at IS NULL;
CREATE INDEX runtime_resource_live_attempt
  ON runtime_resource_leases (attempt_id, acquired_at) WHERE released_at IS NULL;

CREATE TABLE candidate_promotions (
    id              UUID PRIMARY KEY,
    work_id         UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    attempt_id      UUID NOT NULL UNIQUE REFERENCES work_attempts(id) ON DELETE CASCADE,
    repo            TEXT NOT NULL,
    integration_branch TEXT NOT NULL,
    source_commit   TEXT NOT NULL,
    source_tree     TEXT NOT NULL,
    manifest        JSONB NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending'
      CHECK (state IN ('pending', 'completed', 'failed')),
    failure         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ
);
CREATE INDEX candidate_promotions_pending
  ON candidate_promotions (created_at, id) WHERE state = 'pending';

CREATE TABLE immutable_review_targets (
    id              UUID PRIMARY KEY,
    work_id         UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    attempt_id      UUID NOT NULL UNIQUE REFERENCES work_attempts(id) ON DELETE CASCADE,
    content_digest  TEXT NOT NULL,
    uri             TEXT NOT NULL UNIQUE,
    alias_uri       TEXT,
    source_commit   TEXT,
    manifest        JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
