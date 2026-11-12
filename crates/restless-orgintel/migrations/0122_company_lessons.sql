-- Company lessons: a correction the owner made, kept as a standing rule.
--
-- The owner corrected the same kinds of things again and again and nothing held: operating rules
-- were compiled into the binary and the only learning path was brand voice and look. A lesson is
-- one rule in plain words, for the whole company, one team or one agent. Every agent it applies
-- to reads it on each wake. When the same problem shows up again the lesson counts it, so a rule
-- that does not hold is visible instead of quietly re-taught.
--
-- Recoverable coordination state, not authority: a lesson grants nothing and gates nothing.
CREATE TABLE company_lessons (
    id                          UUID PRIMARY KEY,
    -- One or two plain sentences.
    rule                        TEXT NOT NULL CHECK (length(rule) BETWEEN 12 AND 400),
    scope_kind                  TEXT NOT NULL CHECK (scope_kind IN ('company', 'team', 'actor')),
    team_id                     UUID REFERENCES teams(id),
    actor_id                    TEXT REFERENCES actors(id),
    -- The person's message the rule came from. Messages may be compacted, so no foreign key.
    source_message_id           BIGINT,
    recorded_by                 TEXT NOT NULL REFERENCES actors(id),
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Active while NULL. A retired lesson keeps its record and can be restored.
    retired_at                  TIMESTAMPTZ,
    retired_by                  TEXT REFERENCES actors(id),
    retirement_reason           TEXT NOT NULL DEFAULT '',
    -- How many times the same problem came back after the lesson was recorded.
    recurrence_count            INTEGER NOT NULL DEFAULT 0 CHECK (recurrence_count >= 0),
    last_recurred_at            TIMESTAMPTZ,
    last_recurrence_message_id  BIGINT,
    CHECK (
        (scope_kind = 'company' AND team_id IS NULL AND actor_id IS NULL)
        OR (scope_kind = 'team' AND team_id IS NOT NULL AND actor_id IS NULL)
        OR (scope_kind = 'actor' AND team_id IS NULL AND actor_id IS NOT NULL)
    )
);
CREATE INDEX company_lessons_active ON company_lessons (created_at DESC) WHERE retired_at IS NULL;
