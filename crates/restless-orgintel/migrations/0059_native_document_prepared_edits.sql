CREATE TABLE native_document_prepared_edits (
    command_id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES native_documents(id),
    actor_id TEXT NOT NULL REFERENCES actors(id),
    request_json JSONB NOT NULL,
    prepared_json JSONB NOT NULL,
    result_json JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    CHECK (jsonb_typeof(request_json) = 'object'),
    CHECK (jsonb_typeof(prepared_json) = 'object'),
    CHECK ((result_json IS NULL) = (completed_at IS NULL))
);
CREATE INDEX native_document_prepared_edits_document ON native_document_prepared_edits(document_id);
