-- Items waiting on the owner do not go stale in silence (12 November 2026). Exec is asked about
-- one that has waited too long, at most three times, and may raise it again once per ask. The
-- counts live on the handoff so the backoff survives restarts and event compaction. None of
-- these columns is part of the notification epoch: raising a card again is not a new request.
ALTER TABLE owner_handoffs
    ADD COLUMN stale_signal_count INTEGER NOT NULL DEFAULT 0 CHECK (stale_signal_count >= 0),
    ADD COLUMN stale_signalled_at TIMESTAMPTZ,
    ADD COLUMN reraise_count INTEGER NOT NULL DEFAULT 0 CHECK (reraise_count >= 0),
    ADD COLUMN reraised_at TIMESTAMPTZ,
    ADD COLUMN reraise_reason TEXT CHECK (reraise_reason IS NULL OR char_length(reraise_reason) <= 240);
