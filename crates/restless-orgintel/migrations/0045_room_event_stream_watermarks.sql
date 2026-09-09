-- Room replay must not stop collaboration elsewhere in the company. The
-- company-wide event identity remains the stable event/deduplication cursor,
-- but each Room now owns a short transactional watermark row. Writers lock
-- that row before PostgreSQL allocates an event identity; readers take a
-- shared row lock while capturing the committed prefix. An in-flight event in
-- another Room can therefore never force a global event-table lock.
CREATE TABLE room_event_streams (
    room_id                       UUID PRIMARY KEY REFERENCES rooms(id) ON DELETE CASCADE,
    last_event_id                 BIGINT NOT NULL DEFAULT 0 CHECK (last_event_id >= 0),
    compacted_through_event_id    BIGINT NOT NULL DEFAULT 0
                                  CHECK (compacted_through_event_id >= 0),
    updated_at                    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (compacted_through_event_id <= last_event_id)
);

-- Earlier compaction retained only a company-wide floor. Applying that floor
-- to existing Rooms is deliberately conservative: a reconnect may refetch a
-- projection once, but can never silently cross history that may have been
-- removed. Rooms created after this migration start at an exact zero floor.
WITH retained_floor AS (
    SELECT COALESCE(MAX((body->>'through_event_id')::BIGINT), 0) AS event_id
    FROM events
    WHERE kind = 'event.history.compacted.v1'
)
INSERT INTO room_event_streams (
    room_id,
    last_event_id,
    compacted_through_event_id
)
SELECT
    room.id,
    GREATEST(COALESCE(MAX(event.id), 0), retained_floor.event_id),
    retained_floor.event_id
FROM rooms AS room
CROSS JOIN retained_floor
LEFT JOIN events AS event ON event.room_id = room.id
GROUP BY room.id, retained_floor.event_id;

-- This is the only application write primitive for a Room-scoped event. The
-- stream row is locked before INSERT evaluates the events identity default,
-- so a later committed watermark cannot jump over an earlier in-flight event
-- from the same Room. Different Rooms lock different rows.
CREATE OR REPLACE FUNCTION orgintel_append_room_event(
    event_kind TEXT,
    event_room_id UUID,
    event_actor_id TEXT,
    event_message_id BIGINT,
    event_body JSONB
) RETURNS BIGINT AS $$
DECLARE
    created_event_id BIGINT;
BEGIN
    INSERT INTO room_event_streams (room_id)
    VALUES (event_room_id)
    ON CONFLICT (room_id) DO NOTHING;

    PERFORM last_event_id
    FROM room_event_streams
    WHERE room_id = event_room_id
    FOR UPDATE;

    INSERT INTO events (kind, room_id, actor_id, message_id, body)
    VALUES (
        event_kind,
        event_room_id,
        event_actor_id,
        event_message_id,
        event_body
    )
    RETURNING id INTO created_event_id;

    UPDATE room_event_streams
    SET last_event_id = created_event_id,
        updated_at = now()
    WHERE room_id = event_room_id;

    RETURN created_event_id;
END;
$$ LANGUAGE plpgsql;

-- Message creation remains one atomic Message/event/receipt operation, but it
-- now uses the same per-Room ordering primitive as every other Room event.
CREATE OR REPLACE FUNCTION orgintel_room_message_event() RETURNS trigger AS $$
DECLARE
    created_event_id BIGINT;
BEGIN
    IF NEW.room_id IS NOT NULL THEN
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
        SET room_created_event_id = created_event_id
        WHERE id = NEW.id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE INDEX room_event_streams_updated
    ON room_event_streams (updated_at, room_id);
