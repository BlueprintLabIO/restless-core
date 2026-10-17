-- Business observations, not permission to spend or a second payment rail.
CREATE TABLE accounting_records (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('income','expense')),
    description TEXT NOT NULL,
    counterparty TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    amount_minor BIGINT NOT NULL CHECK (amount_minor > 0 AND amount_minor <= 9007199254740991),
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    status TEXT NOT NULL CHECK (status IN ('settled','pending','void')),
    occurred_on DATE NOT NULL,
    evidence TEXT NOT NULL DEFAULT '',
    version BIGINT NOT NULL DEFAULT 1,
    recorded_by TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX accounting_records_date ON accounting_records (occurred_on DESC, id);
