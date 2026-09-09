-- Room Messages are immutable facts. Edits append revisions and deletion
-- appends one tombstone; the mutable columns on `messages` are only bounded
-- read/search projections of those immutable records.

-- Extensions are database-scoped, while OrgIntel migrations run with one
-- company's schema as the whole search path. Install this stateless operator
-- family in `public` once and schema-qualify every use below; no company table
-- or data is resolved through `public`. Early pre-release fixtures may already
-- have installed pg_trgm into their first company schema, so move that one
-- database-global extension instead of leaving later companies unable to use
-- its operator class.
DO $$
DECLARE
  extension_schema TEXT;
BEGIN
  SELECT namespace.nspname INTO extension_schema
  FROM pg_catalog.pg_extension extension
  JOIN pg_catalog.pg_namespace namespace ON namespace.oid=extension.extnamespace
  WHERE extension.extname='pg_trgm';

  IF extension_schema IS NULL THEN
    EXECUTE 'CREATE EXTENSION pg_trgm WITH SCHEMA public';
  ELSIF extension_schema <> 'public' THEN
    EXECUTE 'ALTER EXTENSION pg_trgm SET SCHEMA public';
  END IF;
END;
$$;

ALTER TABLE messages
  ADD COLUMN current_plain_text TEXT,
  ADD COLUMN latest_revision_id UUID,
  ADD COLUMN edited_at TIMESTAMPTZ,
  ADD COLUMN deleted_at TIMESTAMPTZ,
  ADD COLUMN deleted_by_actor_id TEXT REFERENCES actors(id);

UPDATE messages
SET current_plain_text = body
WHERE room_id IS NOT NULL;

ALTER TABLE messages
  ADD CONSTRAINT messages_room_lifecycle_shape CHECK (
    (room_id IS NULL AND current_plain_text IS NULL
      AND latest_revision_id IS NULL AND edited_at IS NULL
      AND deleted_at IS NULL AND deleted_by_actor_id IS NULL)
    OR
    (room_id IS NOT NULL AND current_plain_text IS NOT NULL
      AND octet_length(current_plain_text) <= 65536
      AND ((latest_revision_id IS NULL AND edited_at IS NULL)
        OR (latest_revision_id IS NOT NULL AND edited_at IS NOT NULL))
      AND ((deleted_at IS NULL AND deleted_by_actor_id IS NULL)
        OR (deleted_at IS NOT NULL AND deleted_by_actor_id IS NOT NULL)))
  );

CREATE TABLE room_message_revisions (
  id                    UUID PRIMARY KEY,
  room_id               UUID NOT NULL,
  message_id            BIGINT NOT NULL,
  revision_number       BIGINT NOT NULL CHECK (revision_number > 0),
  editor_actor_id       TEXT NOT NULL REFERENCES actors(id),
  body                   TEXT NOT NULL,
  plain_text             TEXT NOT NULL,
  client_command_id      TEXT NOT NULL,
  client_payload_sha256  TEXT NOT NULL,
  created_event_id       BIGINT NOT NULL,
  created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
  FOREIGN KEY (room_id, message_id)
    REFERENCES messages(room_id, id),
  UNIQUE (message_id, revision_number),
  UNIQUE (room_id, editor_actor_id, client_command_id),
  CHECK (octet_length(body) BETWEEN 1 AND 65536),
  CHECK (octet_length(plain_text) BETWEEN 1 AND 65536),
  CHECK (length(btrim(client_command_id)) BETWEEN 1 AND 128),
  CHECK (client_payload_sha256 ~ '^[0-9a-f]{64}$')
);

ALTER TABLE messages
  ADD CONSTRAINT messages_latest_revision
  FOREIGN KEY (latest_revision_id)
  REFERENCES room_message_revisions(id);

CREATE TABLE room_message_tombstones (
  id                    UUID PRIMARY KEY,
  room_id               UUID NOT NULL,
  message_id            BIGINT NOT NULL UNIQUE,
  deleted_by_actor_id   TEXT NOT NULL REFERENCES actors(id),
  client_command_id     TEXT NOT NULL,
  client_payload_sha256 TEXT NOT NULL,
  created_event_id      BIGINT NOT NULL,
  created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
  FOREIGN KEY (room_id, message_id)
    REFERENCES messages(room_id, id),
  UNIQUE (room_id, deleted_by_actor_id, client_command_id),
  CHECK (length(btrim(client_command_id)) BETWEEN 1 AND 128),
  CHECK (client_payload_sha256 ~ '^[0-9a-f]{64}$')
);

-- The current-text projection is indexed for authorised full-text search.
-- Room-title fuzziness uses pg_trgm's GIN operator class.
CREATE INDEX messages_room_current_text_search
  ON messages USING GIN (to_tsvector('simple', current_plain_text))
  WHERE room_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX rooms_active_title_trgm
  ON rooms USING GIN (title public.gin_trgm_ops)
  WHERE archived_at IS NULL;

-- This is the initial immutable Message projection. The existing Room bridge
-- runs first by trigger-name order for older internal writers; the bridge is a
-- separately scheduled pre-release cutover, after those writers speak Rooms
-- directly. New Room commands also supply this value explicitly.
CREATE OR REPLACE FUNCTION orgintel_room_message_initial_projection()
RETURNS trigger AS $$
BEGIN
  IF NEW.room_id IS NOT NULL AND NEW.current_plain_text IS NULL THEN
    NEW.current_plain_text := NEW.body;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER room_message_initial_projection
  BEFORE INSERT ON messages
  FOR EACH ROW EXECUTE FUNCTION orgintel_room_message_initial_projection();

-- Operational events are compactable hints. Each immutable lifecycle record
-- retains the event cursor returned by the original command so retry responses
-- remain exact even after event compaction.
CREATE UNIQUE INDEX room_message_revisions_created_event
  ON room_message_revisions(created_event_id)
  WHERE created_event_id IS NOT NULL;
CREATE UNIQUE INDEX room_message_tombstones_created_event
  ON room_message_tombstones(created_event_id)
  WHERE created_event_id IS NOT NULL;

CREATE OR REPLACE FUNCTION orgintel_room_message_lifecycle_immutable()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION '% rows are immutable', TG_TABLE_NAME;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER room_message_revisions_immutable
  BEFORE UPDATE OR DELETE ON room_message_revisions
  FOR EACH ROW EXECUTE FUNCTION orgintel_room_message_lifecycle_immutable();
CREATE TRIGGER room_message_tombstones_immutable
  BEFORE UPDATE OR DELETE ON room_message_tombstones
  FOR EACH ROW EXECUTE FUNCTION orgintel_room_message_lifecycle_immutable();
