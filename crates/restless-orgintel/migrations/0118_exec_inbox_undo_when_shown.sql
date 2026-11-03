-- Exec's Inbox changes from the owner's chat get an Undo window the owner can actually use
-- (3 November 2026). A change is held, with no deadline, while the reply that carries it is still
-- streaming; its window opens when that reply is complete and the note is shown (`shown_at`). It
-- then lasts until the owner's next message to Exec, but never less than 30 seconds and never
-- more than two minutes after it was shown. The owner's own clicks keep their five seconds.
ALTER TABLE exec_inbox_actions
    ALTER COLUMN undo_until DROP NOT NULL,
    ALTER COLUMN undo_until DROP DEFAULT,
    ADD COLUMN shown_at TIMESTAMPTZ;
-- Rows saved under 0117 were shown when they were saved.
UPDATE exec_inbox_actions SET shown_at = created_at;
ALTER TABLE exec_inbox_actions
    ADD CONSTRAINT exec_inbox_window_opens_when_shown
        CHECK ((shown_at IS NULL) = (undo_until IS NULL));
