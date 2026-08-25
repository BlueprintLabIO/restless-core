-- One recoverable projection of one Authority-owned inbound fact. The source
-- URI and provider metadata reopen truth; the bounded message is only the
-- organisational context that somebody owes.
CREATE TABLE external_message_sources (
    source_ref          TEXT PRIMARY KEY,
    message_id          BIGINT UNIQUE REFERENCES messages(id),
    provider            TEXT NOT NULL,
    provider_event_id   TEXT NOT NULL,
    provider_email_id   TEXT,
    provider_message_id TEXT,
    provider_thread_id  TEXT,
    source_url          TEXT,
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb,
    projected_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX external_message_provider_event
    ON external_message_sources (provider, provider_event_id);
