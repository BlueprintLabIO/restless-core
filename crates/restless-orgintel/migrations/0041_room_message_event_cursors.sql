-- A retryable Room send returns both the authoritative Message and the event
-- cursor produced by that original command. Operational events are deliberately
-- compactable, so the Message/idempotency row must remember that cursor without
-- a reverse foreign key to `events`.
ALTER TABLE messages
  ADD COLUMN room_created_event_id BIGINT,
  ADD CONSTRAINT messages_room_created_event_shape CHECK (
    room_created_event_id IS NULL OR room_id IS NOT NULL
  );

COMMENT ON COLUMN messages.room_created_event_id IS
  'Durable receipt cursor for the original room.message.created.v1 event; the referenced operational event may have been compacted.';

-- Existing retained Room events have one Message link by the partial unique
-- index introduced in 0037. A cursor that was already compacted cannot be
-- reconstructed honestly. Refuse to activate the new retry contract if that
-- happened to a retryable Message rather than silently returning a new cursor;
-- non-command legacy Messages may remain NULL. Every send after this migration
-- is captured transactionally by the trigger below.
UPDATE messages AS message
SET room_created_event_id = event.id
FROM events AS event
WHERE event.kind = 'room.message.created.v1'
  AND event.message_id = message.id
  AND event.room_id = message.room_id
  AND message.room_created_event_id IS NULL;

DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM messages
    WHERE room_id IS NOT NULL
      AND client_command_id IS NOT NULL
      AND room_created_event_id IS NULL
  ) THEN
    RAISE EXCEPTION
      'cannot recover the original event cursor for a pre-migration retryable Room message';
  END IF;
END;
$$;

CREATE UNIQUE INDEX messages_room_created_event
  ON messages (room_created_event_id)
  WHERE room_created_event_id IS NOT NULL;

-- The event and its durable receipt cursor are written inside the Message
-- insert's transaction. The cursor intentionally has no FK back to the
-- compactable operational stream.
CREATE OR REPLACE FUNCTION orgintel_room_message_event() RETURNS trigger AS $$
DECLARE
  created_event_id BIGINT;
BEGIN
  IF NEW.room_id IS NOT NULL THEN
    INSERT INTO events (
      kind, room_id, actor_id, message_id, body
    ) VALUES (
      'room.message.created.v1',
      NEW.room_id,
      NEW.from_actor,
      NEW.id,
      jsonb_build_object(
        'message_id', NEW.id,
        'parent_message_id', NEW.parent_message_id,
        'thread_root_message_id', COALESCE(NEW.thread_root_message_id, NEW.id)
      )
    ) RETURNING id INTO created_event_id;

    UPDATE messages
    SET room_created_event_id = created_event_id
    WHERE id = NEW.id;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;
