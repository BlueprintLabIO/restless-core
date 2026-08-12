-- T5 · OrgIntel core schema (ARCHITECTURE.md §4.4). Applied once per company
-- schema; the company name is the schema name, set via search_path.
--
-- Deliberately small ontology: actors, goals, commitments, messages,
-- artifact_refs, decisions, events. Recoverable coordination state — NOT a
-- ledger, NOT kernel-governed truth (§4.9). Any table no company writes to
-- during the sprint is deleted (ticket guard).

-- The one state machine in OrgIntel: commitment states are deterministic and
-- enumerable (LLM_CURE.md frame 2). Nowhere else gets an enum.
CREATE TYPE commitment_state AS ENUM (
    'proposed',
    'active',
    'blocked',
    'completed',
    'abandoned'
);

-- Who acts in the company: the singleton Exec, staff, the owner, the system.
CREATE TABLE actors (
    id          TEXT PRIMARY KEY,           -- 'exec', 'staff-frontend', 'owner'
    kind        TEXT NOT NULL,              -- exec | staff | owner | system
    display     TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Where the company is heading. Judgement objects: no state machine, just
-- open (closed_at IS NULL) or closed.
CREATE TABLE goals (
    id          UUID PRIMARY KEY,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    created_by  TEXT NOT NULL REFERENCES actors(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at   TIMESTAMPTZ
);

-- A promised piece of work, owned by one actor, optionally serving a goal.
CREATE TABLE commitments (
    id          UUID PRIMARY KEY,
    goal_id     UUID REFERENCES goals(id),
    owner_id    TEXT NOT NULL REFERENCES actors(id),
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    state       commitment_state NOT NULL DEFAULT 'proposed',
    resolution  TEXT NOT NULL DEFAULT '',   -- why completed/abandoned, what blocks
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Directed messages between actors (salvage: communication.rs concept,
-- universal-command envelope stripped). to_actor NULL = the owner inbox.
CREATE TABLE messages (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    from_actor  TEXT NOT NULL REFERENCES actors(id),
    to_actor    TEXT REFERENCES actors(id),
    body        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at     TIMESTAMPTZ
);
CREATE INDEX messages_inbox ON messages (to_actor, id) WHERE read_at IS NULL;

-- References to work products, never custody (§6.3): a path, repo+commit,
-- worktree+branch, or URL. The referenced thing lives elsewhere.
CREATE TABLE artifact_refs (
    id          UUID PRIMARY KEY,
    kind        TEXT NOT NULL,              -- path | repo | worktree | url
    uri         TEXT NOT NULL,              -- /company/outputs/x, repo@sha, url
    note        TEXT NOT NULL DEFAULT '',
    created_by  TEXT NOT NULL REFERENCES actors(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Decisions the company has made, in prose. Judgement recorded, not enforced.
CREATE TABLE decisions (
    id          UUID PRIMARY KEY,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    decided_by  TEXT NOT NULL REFERENCES actors(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Operational stream for UI, debugging, awareness. May be compacted,
-- repaired, regenerated (§4.4) — nothing here is constitutional truth.
CREATE TABLE events (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind        TEXT NOT NULL,              -- 'wake', 'spawn', 'effect', ...
    actor_id    TEXT,
    body        JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX events_kind_id ON events (kind, id);
