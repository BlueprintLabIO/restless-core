-- Bounded, durable protocol-only repairs for a productive Work Attempt.
-- Only hashes and counts are retained; raw model output remains in the
-- existing restricted execution diagnostics and never enters this ledger.

CREATE TABLE completion_protocol_repair_budgets (
    attempt_id UUID PRIMARY KEY REFERENCES work_attempts(id) ON DELETE CASCADE,
    repairs_total INTEGER NOT NULL DEFAULT 0 CHECK (repairs_total >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE completion_protocol_repair_fingerprints (
    attempt_id UUID NOT NULL REFERENCES completion_protocol_repair_budgets(attempt_id) ON DELETE CASCADE,
    failure_fingerprint_sha256 TEXT NOT NULL CHECK (failure_fingerprint_sha256 ~ '^[0-9a-f]{64}$'),
    repairs INTEGER NOT NULL DEFAULT 0 CHECK (repairs >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (attempt_id, failure_fingerprint_sha256)
);

CREATE TABLE completion_protocol_repair_reservations (
    command_id UUID PRIMARY KEY,
    attempt_id UUID NOT NULL REFERENCES completion_protocol_repair_budgets(attempt_id) ON DELETE CASCADE,
    failure_fingerprint_sha256 TEXT NOT NULL CHECK (failure_fingerprint_sha256 ~ '^[0-9a-f]{64}$'),
    admitted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX completion_protocol_repair_attempt_history
    ON completion_protocol_repair_reservations (attempt_id, admitted_at DESC);
