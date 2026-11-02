-- Exec manages the owner's Inbox from their chat (owner decision, 3 November 2026). Each row is
-- one thing Exec did on the owner's word: it names the owner's message, waits out the same
-- five-second Undo window as the owner's own Inbox actions, and is then applied through the
-- item's own decision path. An `add` row is itself an Inbox item until it is set aside.
CREATE TABLE exec_inbox_actions (
    id UUID PRIMARY KEY,
    item_id TEXT NOT NULL CHECK (octet_length(item_id) BETWEEN 1 AND 512),
    item_title TEXT NOT NULL CHECK (octet_length(item_title) <= 2000),
    verb TEXT NOT NULL
        CHECK (verb IN ('accept','decline','answer','snooze','dismiss','move','add')),
    note TEXT NOT NULL DEFAULT '' CHECK (octet_length(note) <= 16384),
    -- What the decision path needs to find the item again: its source kind and references.
    target JSONB NOT NULL DEFAULT '{}'::jsonb,
    snooze_until TIMESTAMPTZ,
    position INTEGER CHECK (position IS NULL OR position >= 0),
    source_message_id BIGINT NOT NULL,
    owner_words TEXT NOT NULL,
    undo_until TIMESTAMPTZ NOT NULL DEFAULT (clock_timestamp() + INTERVAL '5 seconds'),
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending','canceled','applied','failed')),
    problem TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ,
    CHECK (verb <> 'snooze' OR snooze_until IS NOT NULL),
    CHECK (verb <> 'move' OR position IS NOT NULL)
);
CREATE UNIQUE INDEX one_pending_exec_action_per_item
    ON exec_inbox_actions(item_id) WHERE state='pending';
CREATE INDEX due_exec_inbox_actions
    ON exec_inbox_actions(undo_until,id) WHERE state='pending';
CREATE INDEX exec_inbox_actions_recent
    ON exec_inbox_actions(created_at DESC);
