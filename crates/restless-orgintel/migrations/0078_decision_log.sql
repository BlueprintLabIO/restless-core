-- A structured decision a model made for the company: what it was asked about, the answers with
-- their probabilities, the action taken, and (when the company later shows the right answer) the
-- outcome. Shadow decisions change nothing; they are kept so a decision earns its switch-on from
-- the company's own record. Recoverable operational state, not governance.
CREATE TABLE decision_log (
  id BIGSERIAL PRIMARY KEY,
  kind TEXT NOT NULL CHECK (length(kind) BETWEEN 1 AND 64),
  subject TEXT NOT NULL CHECK (length(subject) BETWEEN 1 AND 200),
  mode TEXT NOT NULL CHECK (mode IN ('shadow', 'active')),
  model TEXT NOT NULL CHECK (length(model) BETWEEN 1 AND 160),
  answers JSONB NOT NULL,
  choice TEXT NOT NULL CHECK (length(choice) BETWEEN 1 AND 200),
  latency_ms INTEGER NOT NULL CHECK (latency_ms >= 0),
  outcome TEXT CHECK (outcome IS NULL OR length(outcome) BETWEEN 1 AND 200),
  outcome_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (kind, subject)
);
CREATE INDEX decision_log_kind_time ON decision_log (kind, created_at DESC);
