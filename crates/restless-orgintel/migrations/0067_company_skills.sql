-- Sprint 55. A company skill is an ordinary Runtime directory in the open
-- `SKILL.md` format. OrgIntel records only what the company decided about it:
-- which exact content was observed (the SKILL.md digest), its disposition and
-- who may use it. The package body is never copied into governed state.
CREATE TABLE skills (
    name          TEXT PRIMARY KEY CHECK (name ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    description   TEXT NOT NULL DEFAULT '',
    -- builtin: /opt/restless/skills; company: /company/skills;
    -- project: a worktree `.agents/skills`; candidate: imported, not accepted.
    source        TEXT NOT NULL CHECK (source IN ('builtin', 'company', 'project', 'candidate')),
    path          TEXT NOT NULL,
    digest        TEXT NOT NULL,
    has_scripts   BOOLEAN NOT NULL DEFAULT FALSE,
    disposition   TEXT NOT NULL CHECK (disposition IN ('candidate', 'accepted', 'retired')),
    added_by      TEXT REFERENCES actors(id),
    origin_url    TEXT,
    origin_ref    TEXT,
    observed_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Resolution order is actor, then team, then company. An explicit `false`
-- row removes a skill that a wider scope granted.
CREATE TABLE skill_assignments (
    skill_name   TEXT NOT NULL REFERENCES skills(name) ON DELETE CASCADE,
    scope        TEXT NOT NULL CHECK (scope IN ('company', 'team', 'actor')),
    scope_id     TEXT NOT NULL DEFAULT '',
    enabled      BOOLEAN NOT NULL,
    assigned_by  TEXT NOT NULL REFERENCES actors(id),
    assigned_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (skill_name, scope, scope_id),
    CHECK ((scope = 'company') = (scope_id = ''))
);

-- An explicit skill selection is a structured projection over the one
-- authoritative Message, like a mention. The digest pins what was selected.
CREATE TABLE message_skill_selections (
    message_id  BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    skill_name  TEXT NOT NULL,
    digest      TEXT NOT NULL,
    PRIMARY KEY (message_id, skill_name)
);

-- The same selection carried onto Work survives delegation and retries: every
-- Attempt of the Work reads it, whichever harness runs the Attempt.
CREATE TABLE work_skill_selections (
    work_id            UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    skill_name         TEXT NOT NULL,
    digest             TEXT NOT NULL,
    source_message_id  BIGINT REFERENCES messages(id),
    selected_by        TEXT NOT NULL REFERENCES actors(id),
    selected_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (work_id, skill_name)
);

-- `/loop <interval>` is a bounded interval recurrence. It is still only a
-- durable time fact addressed to an actor; it stores no command.
ALTER TABLE schedules ADD COLUMN interval_seconds INTEGER;
ALTER TABLE schedules DROP CONSTRAINT schedules_recurrence_shape;
ALTER TABLE schedules ADD CONSTRAINT schedules_recurrence_shape CHECK (
    (recurrence IS NULL AND timezone IS NULL AND local_time IS NULL AND interval_seconds IS NULL)
    OR
    (recurrence = 'weekdays' AND timezone IS NOT NULL AND local_time IS NOT NULL
        AND work_id IS NULL AND interval_seconds IS NULL)
    OR
    (recurrence = 'interval' AND interval_seconds BETWEEN 300 AND 2592000
        AND timezone IS NULL AND local_time IS NULL AND work_id IS NULL)
);

-- Repeating the same `/loop` returns the existing live schedule.
CREATE UNIQUE INDEX one_live_interval_schedule
  ON schedules (actor_id, interval_seconds, reason)
  WHERE recurrence = 'interval' AND cancelled_at IS NULL;
