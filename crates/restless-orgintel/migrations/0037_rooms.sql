-- Human Rooms are an additive projection over the existing `messages` table.
-- `messages.id` remains the one durable message identity used by Work feedback,
-- actor inboxes and owner conversations. Rooms add audience, reply-tree and
-- participant-relative read state without creating a second chat history.

CREATE TYPE room_kind AS ENUM ('company', 'group', 'direct');
CREATE TYPE room_participant_role AS ENUM ('owner', 'member');

CREATE TABLE rooms (
    id            UUID PRIMARY KEY,
    kind          room_kind NOT NULL,
    title         TEXT NOT NULL,
    created_by    TEXT NOT NULL REFERENCES actors(id),
    -- Stable only for singleton/company and actor-pair/direct rooms. Ordinary
    -- group rooms deliberately have no inferred identity.
    canonical_key TEXT UNIQUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at   TIMESTAMPTZ,
    CHECK (length(btrim(title)) BETWEEN 1 AND 160),
    CHECK (
      (kind IN ('company', 'direct') AND canonical_key IS NOT NULL)
      OR (kind = 'group' AND canonical_key IS NULL)
    )
);

CREATE TABLE room_participants (
    room_id    UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    actor_id   TEXT NOT NULL REFERENCES actors(id),
    role       room_participant_role NOT NULL DEFAULT 'member',
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at    TIMESTAMPTZ,
    PRIMARY KEY (room_id, actor_id),
    CHECK (left_at IS NULL OR left_at >= joined_at)
);
CREATE INDEX room_participants_actor_active
    ON room_participants (actor_id, room_id) WHERE left_at IS NULL;

ALTER TABLE messages
    ADD COLUMN room_id UUID REFERENCES rooms(id),
    ADD COLUMN parent_message_id BIGINT,
    ADD COLUMN thread_root_message_id BIGINT,
    ADD COLUMN client_command_id TEXT,
    ADD COLUMN client_payload_sha256 TEXT,
    ADD CONSTRAINT messages_room_identity UNIQUE (room_id, id),
    ADD CONSTRAINT messages_client_command_shape CHECK (
      (client_command_id IS NULL AND client_payload_sha256 IS NULL)
      OR (
        room_id IS NOT NULL
        AND length(btrim(client_command_id)) BETWEEN 1 AND 128
        AND client_payload_sha256 ~ '^[0-9a-f]{64}$'
      )
    ),
    ADD CONSTRAINT messages_reply_has_room CHECK (
      (parent_message_id IS NULL AND thread_root_message_id IS NULL)
      OR room_id IS NOT NULL
    ),
    ADD CONSTRAINT messages_reply_shape CHECK (
      (parent_message_id IS NULL AND thread_root_message_id IS NULL)
      OR (
        parent_message_id IS NOT NULL
        AND thread_root_message_id IS NOT NULL
        AND thread_root_message_id <= parent_message_id
        AND parent_message_id < id
      )
    );

-- A reply can only point into its own Room. New ids are monotonic and the
-- application only points backwards, so a reply tree cannot become cyclic.
ALTER TABLE messages
    ADD CONSTRAINT messages_parent_in_room
      FOREIGN KEY (room_id, parent_message_id)
      REFERENCES messages(room_id, id),
    ADD CONSTRAINT messages_thread_root_in_room
      FOREIGN KEY (room_id, thread_root_message_id)
      REFERENCES messages(room_id, id);

CREATE UNIQUE INDEX messages_room_client_command
    ON messages (room_id, from_actor, client_command_id)
    WHERE client_command_id IS NOT NULL;
CREATE INDEX messages_room_cursor ON messages (room_id, id);
CREATE INDEX messages_room_thread
    ON messages (room_id, thread_root_message_id, id)
    WHERE thread_root_message_id IS NOT NULL;

CREATE TABLE room_read_cursors (
    room_id               UUID NOT NULL,
    actor_id              TEXT NOT NULL,
    last_read_message_id  BIGINT,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (room_id, actor_id),
    FOREIGN KEY (room_id, actor_id)
      REFERENCES room_participants(room_id, actor_id) ON DELETE CASCADE,
    FOREIGN KEY (room_id, last_read_message_id)
      REFERENCES messages(room_id, id)
);

-- A model-backed Actor can have a separate focused conversation with each
-- human participant. The historical Actor-level cursor remains as the owner
-- compatibility projection, while new human entry uses this direct-Room
-- boundary so one person's "new focus" never truncates another person's
-- context. NULL is the pre-first-message boundary.
CREATE TABLE room_conversation_focus (
    room_id           UUID NOT NULL,
    target_actor_id   TEXT NOT NULL,
    after_message_id  BIGINT,
    started_at        TIMESTAMPTZ,
    PRIMARY KEY (room_id, target_actor_id),
    FOREIGN KEY (room_id, target_actor_id)
      REFERENCES room_participants(room_id, actor_id) ON DELETE CASCADE,
    FOREIGN KEY (room_id, after_message_id)
      REFERENCES messages(room_id, id)
);

-- Collaboration delivery extends the existing compactable operational event
-- stream. `events.id` is the one reconnect cursor; nullable Room/Message
-- coordinates make private filtering indexed without introducing a parallel
-- event store. Event bodies remain bounded refetch hints, never message truth.
ALTER TABLE events
    ADD COLUMN room_id UUID REFERENCES rooms(id) ON DELETE CASCADE,
    ADD COLUMN message_id BIGINT,
    ADD CONSTRAINT events_collaboration_shape CHECK (
      message_id IS NULL OR room_id IS NOT NULL
    ),
    ADD CONSTRAINT events_room_message
      FOREIGN KEY (room_id, message_id)
      REFERENCES messages(room_id, id);
CREATE UNIQUE INDEX events_room_message_created
    ON events (message_id)
    WHERE kind = 'room.message.created.v1';
CREATE INDEX events_collaboration_room_cursor
    ON events (room_id, id) WHERE room_id IS NOT NULL;

-- Compatibility migration: every existing point-to-point message is attached
-- to one deterministic direct Room. `to_actor IS NULL` is the established
-- owner-inbox spelling, so its peer is the stable `owner` actor. If a damaged
-- historical schema lacks that actor, the row stays readable through the
-- legacy path and can be repaired explicitly rather than being misattributed.
WITH legacy_pairs AS (
    SELECT DISTINCT
      LEAST(message.from_actor, COALESCE(message.to_actor, 'owner')) AS first_actor,
      GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner')) AS second_actor
    FROM messages message
), valid_pairs AS (
    SELECT pair.*
    FROM legacy_pairs pair
    JOIN actors first_actor ON first_actor.id = pair.first_actor
    JOIN actors second_actor ON second_actor.id = pair.second_actor
)
INSERT INTO rooms (id, kind, title, created_by, canonical_key)
SELECT
  md5(
    'restless:room:direct:' || octet_length(first_actor) || ':' || first_actor || ':' ||
    octet_length(second_actor) || ':' || second_actor
  )::uuid,
  'direct',
  CASE
    WHEN first_actor = second_actor THEN 'Notes for ' || first_actor
    ELSE first_actor || ' / ' || second_actor
  END,
  first_actor,
  'direct:' || octet_length(first_actor) || ':' || first_actor || ':' ||
    octet_length(second_actor) || ':' || second_actor
FROM valid_pairs
ON CONFLICT (canonical_key) DO NOTHING;

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
    JOIN actors actor ON actor.id = participant.actor_id
)
INSERT INTO room_participants (room_id, actor_id, role)
SELECT
  room_id,
  actor_id,
  CASE WHEN actor_id = created_by THEN 'owner'::room_participant_role
       ELSE 'member'::room_participant_role END
FROM legacy_room_actors
ON CONFLICT (room_id, actor_id) DO NOTHING;

UPDATE messages message
SET room_id = room.id
FROM rooms room
WHERE message.room_id IS NULL
  AND room.canonical_key =
    'direct:' ||
    octet_length(LEAST(message.from_actor, COALESCE(message.to_actor, 'owner'))) || ':' ||
    LEAST(message.from_actor, COALESCE(message.to_actor, 'owner')) || ':' ||
    octet_length(GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner'))) || ':' ||
    GREATEST(message.from_actor, COALESCE(message.to_actor, 'owner'));

-- Every future legacy writer (including Work/review/scheduler paths that write
-- `messages` directly) is bridged in the same transaction. A Room-aware writer
-- supplies `room_id` and bypasses this compatibility projection.
CREATE OR REPLACE FUNCTION orgintel_bridge_message_room() RETURNS trigger AS $$
DECLARE
  peer_actor TEXT;
  first_actor TEXT;
  second_actor TEXT;
  room_key TEXT;
  bridged_room_id UUID;
  room_creator TEXT;
BEGIN
  IF NEW.room_id IS NOT NULL THEN
    RETURN NEW;
  END IF;

  peer_actor := COALESCE(NEW.to_actor, 'owner');
  IF NOT EXISTS (SELECT 1 FROM actors WHERE id = peer_actor) THEN
    RETURN NEW;
  END IF;

  first_actor := LEAST(NEW.from_actor, peer_actor);
  second_actor := GREATEST(NEW.from_actor, peer_actor);
  room_key :=
    'direct:' || octet_length(first_actor) || ':' || first_actor || ':' ||
    octet_length(second_actor) || ':' || second_actor;
  bridged_room_id := md5('restless:room:' || room_key)::uuid;

  INSERT INTO rooms (id, kind, title, created_by, canonical_key)
  VALUES (
    bridged_room_id,
    'direct',
    CASE WHEN first_actor = second_actor THEN 'Notes for ' || first_actor
         ELSE first_actor || ' / ' || second_actor END,
    first_actor,
    room_key
  )
  ON CONFLICT (canonical_key) DO NOTHING;

  SELECT id, created_by INTO bridged_room_id, room_creator
  FROM rooms WHERE canonical_key = room_key;

  INSERT INTO room_participants (room_id, actor_id, role)
  SELECT
    bridged_room_id,
    participant.actor_id,
    CASE WHEN participant.actor_id = room_creator
         THEN 'owner'::room_participant_role
         ELSE 'member'::room_participant_role END
  FROM (
    SELECT DISTINCT actor_id
    FROM (VALUES (NEW.from_actor), (peer_actor)) AS actors(actor_id)
  ) participant
  ON CONFLICT (room_id, actor_id) DO UPDATE SET left_at = NULL;

  NEW.room_id := bridged_room_id;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER message_room_compatibility_bridge
  BEFORE INSERT ON messages
  FOR EACH ROW EXECUTE FUNCTION orgintel_bridge_message_room();

-- The operational event and its message share one commit boundary even when an older
-- internal caller inserts into `messages` directly.
CREATE OR REPLACE FUNCTION orgintel_room_message_event() RETURNS trigger AS $$
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
    );
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER room_message_event
  AFTER INSERT ON messages
  FOR EACH ROW EXECUTE FUNCTION orgintel_room_message_event();
