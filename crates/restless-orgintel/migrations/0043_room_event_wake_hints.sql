-- Room live replay is driven by durable `events`. PostgreSQL notifications are
-- a body-free latency hint only: they may be lost, duplicated, or delivered
-- after reconnect, and consumers always reread the authorized event page.
-- Keep this trigger separate from `orgintel_notify`, whose shape is shared by
-- the scheduler and has changed across earlier migrations.
CREATE OR REPLACE FUNCTION orgintel_notify_room_event() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('restless_orgintel', json_build_object(
    'company', TG_TABLE_SCHEMA,
    'kind', 'room_event',
    'body', json_build_object(
      'room_id', NEW.room_id,
      'event_id', NEW.id,
      'scope', CASE
        WHEN NEW.kind = 'event.history.compacted.v1' THEN 'all_rooms'
        ELSE 'room'
      END
    )
  )::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER room_event_notify
  AFTER INSERT ON events
  FOR EACH ROW
  WHEN (NEW.room_id IS NOT NULL OR NEW.kind = 'event.history.compacted.v1')
  EXECUTE FUNCTION orgintel_notify_room_event();
