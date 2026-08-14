-- Fix the T6 notify payloads: the company is the TRIGGER'S table schema
-- (TG_TABLE_SCHEMA), not the writer's current_schema — a writer with a
-- different search_path (psql -c 'update cosmon.commitments ...') must
-- still attribute the event to the company whose row landed.

CREATE OR REPLACE FUNCTION orgintel_notify() RETURNS trigger AS $$
BEGIN
  IF TG_TABLE_NAME = 'commitments' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', TG_TABLE_SCHEMA,
      'kind', 'commitment_completed',
      'body', json_build_object(
        'commitment_id', NEW.id,
        'title', NEW.title,
        'owner', NEW.owner_id
      )
    )::text);
  ELSIF TG_TABLE_NAME = 'messages' THEN
    PERFORM pg_notify('restless_orgintel', json_build_object(
      'company', TG_TABLE_SCHEMA,
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
