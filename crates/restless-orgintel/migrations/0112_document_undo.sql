-- Human decisions on Native Document review/collaboration requests. The document's
-- own transaction records the result; these rows never contain an HTTP request or credential.
CREATE TABLE owner_document_decisions (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
    request_id UUID NOT NULL,
    principal JSONB NOT NULL,
    decision JSONB NOT NULL,
    request_snapshot JSONB NOT NULL,
    comment_command_id UUID NOT NULL,
    undo_until TIMESTAMPTZ NOT NULL DEFAULT (clock_timestamp() + INTERVAL '5 seconds'),
    state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','canceled','applied','failed')),
    problem TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ,
    seen_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX one_pending_decision_per_document_request
    ON owner_document_decisions(document_id,request_id,(decision->>'kind')) WHERE state='pending';
CREATE INDEX due_document_decisions ON owner_document_decisions(undo_until,id) WHERE state='pending';
