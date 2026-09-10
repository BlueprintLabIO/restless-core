-- Fleet is authoritative for the desired Runtime revision; Core persists the
-- highest authenticated revision it has observed for this exact cell/runtime
-- pair so stale activity requests cannot be echoed as current after restart.
CREATE TABLE runtime_activity_revision (
  cell_id UUID PRIMARY KEY REFERENCES company_access_identity(cell_id) ON DELETE RESTRICT,
  runtime_id TEXT NOT NULL CHECK (length(runtime_id) BETWEEN 1 AND 160),
  desired_revision BIGINT NOT NULL CHECK (desired_revision > 0),
  observed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
