-- A person's five-second Undo window on an existing handoff. These are pending
-- human decisions, not permission grants or queued HTTP requests. The existing
-- handoff transaction applies the decision and records its receipt together.
CREATE TABLE owner_handoff_decisions (
    id UUID PRIMARY KEY,
    handoff_id UUID NOT NULL REFERENCES owner_handoffs(id) ON DELETE CASCADE,
    principal JSONB NOT NULL,
    expected_version JSONB NOT NULL,
    decision JSONB NOT NULL,
    undo_until TIMESTAMPTZ NOT NULL DEFAULT (clock_timestamp() + INTERVAL '5 seconds'),
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending','canceled','applied','failed')),
    problem TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ,
    seen_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX one_pending_decision_per_handoff
    ON owner_handoff_decisions(handoff_id) WHERE state='pending';
CREATE INDEX due_handoff_decisions
    ON owner_handoff_decisions(undo_until,id) WHERE state='pending';
