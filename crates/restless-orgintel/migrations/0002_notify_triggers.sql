-- T6: event-driven wakeups. A dependent result LANDS when its row lands —
-- the trigger condition (state transition to completed, mail inserted) is
-- deterministic and enumerable, so the database is the honest home for it:
-- no writer (OrgIntel method, psql, future tooling) can bypass it.
-- The payload carries the company (schema) and the commitment owner so the
-- scheduler can wake the right actor — and skip the Exec's own completions,
-- which need no wake (it just decided).

CREATE OR REPLACE FUNCTION orgintel_notify() RETURNS trigger AS $$
BEGIN
  IF TG_TABLE_NAME = 'commitments' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', current_schema,
      'kind', 'commitment_completed',
      'body', json_build_object(
        'commitment_id', NEW.id,
        'title', NEW.title,
        'owner', NEW.owner_id
      )
    )::text);
  ELSIF TG_TABLE_NAME = 'messages' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', current_schema,
      'kind', 'message',
      'body', json_build_object(
        'message_id', NEW.id,
        'to', NEW.to_actor,
        'from', NEW.from_actor
      )
    )::text);
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER commitment_completed_notify
  AFTER UPDATE OF state ON commitments
  FOR EACH ROW WHEN (NEW.state = 'completed')
  EXECUTE FUNCTION orgintel_notify();

CREATE TRIGGER message_notify
  AFTER INSERT ON messages
  FOR EACH ROW WHEN (NEW.to_actor IS NOT NULL)
  EXECUTE FUNCTION orgintel_notify();
