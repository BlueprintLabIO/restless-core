-- Sprint 39: a durable session identifies the certified harness that actually
-- ran it. Configuration is intent; these rows are the launch observation.
ALTER TABLE work_attempts
  ADD COLUMN harness TEXT,
  ADD COLUMN harness_build TEXT,
  ADD COLUMN harness_transport TEXT,
  ADD COLUMN harness_capabilities JSONB;

CREATE TABLE agent_sessions (
  launch_id TEXT PRIMARY KEY,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  responsibility TEXT NOT NULL,
  work_id UUID REFERENCES work(id),
  attempt_id UUID REFERENCES work_attempts(id),
  harness TEXT NOT NULL,
  harness_build TEXT NOT NULL,
  transport TEXT NOT NULL,
  model TEXT NOT NULL,
  configured_effort TEXT NOT NULL,
  provider_session_id TEXT NOT NULL,
  capabilities JSONB NOT NULL,
  resumed BOOLEAN NOT NULL,
  reconstructed BOOLEAN NOT NULL,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_sessions_attempt_idx ON agent_sessions(attempt_id)
  WHERE attempt_id IS NOT NULL;
