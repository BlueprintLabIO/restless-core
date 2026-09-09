-- Every durable Message now names its Room at the write boundary. This is a
-- pre-release cutover: migrate the retained data, remove database inference,
-- and make an omitted Room identity an immediate integrity error.

DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM messages message
    LEFT JOIN actors peer
      ON peer.id=COALESCE(message.to_actor, 'owner')
    WHERE message.room_id IS NULL AND peer.id IS NULL
  ) THEN
    RAISE EXCEPTION
      'cannot assign explicit Rooms: a retained Message has no durable recipient Actor';
  END IF;
END;
$$;

WITH legacy_pairs AS (
  SELECT DISTINCT
    LEAST(message.from_actor, COALESCE(message.to_actor, 'owner')) AS first_actor,
    GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner')) AS second_actor
  FROM messages message
  WHERE message.room_id IS NULL
), created_rooms AS (
  INSERT INTO rooms (id,kind,title,created_by,canonical_key)
  SELECT
    md5(
      'restless:room:direct:' || octet_length(first_actor) || ':' || first_actor || ':' ||
      octet_length(second_actor) || ':' || second_actor
    )::uuid,
    'direct',
    CASE WHEN first_actor=second_actor THEN 'Notes for ' || first_actor
         ELSE first_actor || ' / ' || second_actor END,
    first_actor,
    'direct:' || octet_length(first_actor) || ':' || first_actor || ':' ||
      octet_length(second_actor) || ':' || second_actor
  FROM legacy_pairs
  ON CONFLICT (canonical_key) DO NOTHING
  RETURNING id
)
SELECT count(*) FROM created_rooms;

WITH legacy_room_actors AS (
  SELECT DISTINCT
    room.id AS room_id,
    room.created_by,
    participant.actor_id
  FROM messages message
  JOIN rooms room ON room.canonical_key =
    'direct:' ||
    octet_length(LEAST(message.from_actor, COALESCE(message.to_actor, 'owner'))) || ':' ||
    LEAST(message.from_actor, COALESCE(message.to_actor, 'owner')) || ':' ||
    octet_length(GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner'))) || ':' ||
    GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner'))
  CROSS JOIN LATERAL (
    VALUES (message.from_actor), (COALESCE(message.to_actor, 'owner'))
  ) AS participant(actor_id)
  WHERE message.room_id IS NULL
)
INSERT INTO room_participants (room_id,actor_id,role)
SELECT
  room_id,
  actor_id,
  CASE WHEN actor_id=created_by THEN 'owner'::room_participant_role
       ELSE 'member'::room_participant_role END
FROM legacy_room_actors
ON CONFLICT (room_id,actor_id) DO UPDATE
SET role=EXCLUDED.role,joined_at=now(),left_at=NULL
WHERE room_participants.left_at IS NOT NULL
   OR room_participants.role<>EXCLUDED.role;

UPDATE messages message
SET room_id=room.id,
    current_plain_text=COALESCE(message.current_plain_text, message.body)
FROM rooms room
WHERE message.room_id IS NULL
  AND room.archived_at IS NULL
  AND room.canonical_key =
    'direct:' ||
    octet_length(LEAST(message.from_actor, COALESCE(message.to_actor, 'owner'))) || ':' ||
    LEAST(message.from_actor, COALESCE(message.to_actor, 'owner')) || ':' ||
    octet_length(GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner'))) || ':' ||
    GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner'));

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM messages WHERE room_id IS NULL) THEN
    RAISE EXCEPTION
      'cannot enforce explicit Rooms: at least one retained Message could not be assigned';
  END IF;
END;
$$;

DROP TRIGGER message_room_compatibility_bridge ON messages;
DROP FUNCTION orgintel_bridge_message_room();

ALTER TABLE messages
  ALTER COLUMN room_id SET NOT NULL,
  DROP CONSTRAINT messages_room_lifecycle_shape,
  ADD CONSTRAINT messages_room_lifecycle_shape CHECK (
    current_plain_text IS NOT NULL
    AND octet_length(current_plain_text) <= 65536
    AND ((latest_revision_id IS NULL AND edited_at IS NULL)
      OR (latest_revision_id IS NOT NULL AND edited_at IS NOT NULL))
    AND ((deleted_at IS NULL AND deleted_by_actor_id IS NULL)
      OR (deleted_at IS NOT NULL AND deleted_by_actor_id IS NOT NULL))
  );

-- Initial body and event projections now assume the schema invariant rather
-- than carrying a branch for an impossible nullable Room.
CREATE OR REPLACE FUNCTION orgintel_room_message_initial_projection()
RETURNS trigger AS $$
BEGIN
  IF NEW.current_plain_text IS NULL THEN
    NEW.current_plain_text := NEW.body;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION orgintel_room_message_event() RETURNS trigger AS $$
DECLARE
  created_event_id BIGINT;
BEGIN
  created_event_id := orgintel_append_room_event(
    'room.message.created.v1',
    NEW.room_id,
    NEW.from_actor,
    NEW.id,
    jsonb_build_object(
      'message_id', NEW.id,
      'parent_message_id', NEW.parent_message_id,
      'thread_root_message_id', COALESCE(NEW.thread_root_message_id, NEW.id)
    )
  );

  UPDATE messages
  SET room_created_event_id=created_event_id
  WHERE id=NEW.id;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;
