-- A request to co-edit is independent of accepting an immutable named version.
CREATE TABLE native_document_collaboration_requests (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES native_documents(id),
    requested_by_actor_id TEXT NOT NULL REFERENCES actors(id),
    summary TEXT NOT NULL CHECK (length(summary) BETWEEN 1 AND 4000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    resolved_by_actor_id TEXT REFERENCES actors(id),
    CHECK ((resolved_at IS NULL) = (resolved_by_actor_id IS NULL))
);
CREATE INDEX native_document_collaboration_requests_pending
    ON native_document_collaboration_requests(document_id,created_at) WHERE resolved_at IS NULL;
