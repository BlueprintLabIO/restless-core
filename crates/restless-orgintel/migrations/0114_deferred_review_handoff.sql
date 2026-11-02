-- Deferred admission belongs to the handoff: events can be compacted.
ALTER TABLE owner_handoffs
    ADD COLUMN deferred_review_attempt_id UUID REFERENCES work_attempts(id),
    ADD COLUMN deferred_review_by TEXT REFERENCES actors(id),
    ADD COLUMN deferred_review_fingerprint TEXT,
    ADD CONSTRAINT owner_handoff_deferred_review_fields CHECK (
        (deferred_review_attempt_id IS NULL AND deferred_review_by IS NULL AND deferred_review_fingerprint IS NULL)
        OR (deferred_review_attempt_id IS NOT NULL AND deferred_review_by IS NOT NULL AND deferred_review_fingerprint IS NOT NULL)
    );

-- A transfer invalidates the previous assignee's deferred request, including
-- transfers performed by team or membership maintenance rather than escalation.
CREATE FUNCTION clear_deferred_review_on_transfer() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.assigned_to IS DISTINCT FROM OLD.assigned_to THEN
        NEW.deferred_review_attempt_id := NULL;
        NEW.deferred_review_by := NULL;
        NEW.deferred_review_fingerprint := NULL;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER owner_handoff_clear_deferred_on_transfer
    BEFORE UPDATE OF assigned_to ON owner_handoffs
    FOR EACH ROW EXECUTE FUNCTION clear_deferred_review_on_transfer();
