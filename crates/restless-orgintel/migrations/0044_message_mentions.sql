-- Mentions are a structured, recipient-relative projection over the one
-- authoritative Message. They are not another mailbox, chat history, or
-- Attention lifecycle: unresolved rows are the durable fact from which the
-- recipient's attention and Runtime wake are derived.

CREATE TYPE message_mention_kind AS ENUM ('direct', 'exec');

CREATE TABLE message_mentions (
    id                              UUID PRIMARY KEY,
    room_id                         UUID NOT NULL,
    message_id                      BIGINT NOT NULL,
    thread_root_message_id          BIGINT NOT NULL,
    mentioned_actor_id              TEXT NOT NULL REFERENCES actors(id),
    kind                            message_mention_kind NOT NULL,
    work_id                         UUID REFERENCES work(id),
    why_this_actor                  TEXT,
    expected_response               TEXT,
    recommendation                  TEXT,
    alternatives                    JSONB NOT NULL DEFAULT '[]'::jsonb,
    evidence                        JSONB NOT NULL DEFAULT '[]'::jsonb,
    uncertainty                     TEXT,
    affected_scope                  TEXT,
    deadline_at                     TIMESTAMPTZ,
    fallback                        TEXT,
    independent_work_can_continue   BOOLEAN NOT NULL DEFAULT TRUE,
    created_event_id                BIGINT NOT NULL,
    claim_token                     UUID,
    claimed_at                      TIMESTAMPTZ,
    claimed_until                   TIMESTAMPTZ,
    resolution_message_id           BIGINT,
    resolution_claim_token          UUID,
    resolved_event_id               BIGINT,
    cancelled_event_id              BIGINT,
    cancelled_by                    TEXT REFERENCES actors(id),
    cancellation_reason             TEXT,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at                     TIMESTAMPTZ,
    cancelled_at                    TIMESTAMPTZ,
    UNIQUE (message_id, mentioned_actor_id),
    UNIQUE (id, mentioned_actor_id),
    UNIQUE (created_event_id),
    UNIQUE (claim_token),
    UNIQUE (resolution_message_id),
    UNIQUE (resolution_claim_token),
    UNIQUE (resolved_event_id),
    UNIQUE (cancelled_event_id),
    FOREIGN KEY (room_id, message_id)
      REFERENCES messages(room_id, id),
    FOREIGN KEY (room_id, thread_root_message_id)
      REFERENCES messages(room_id, id),
    FOREIGN KEY (room_id, resolution_message_id)
      REFERENCES messages(room_id, id),
    CHECK (
      (kind = 'exec' AND mentioned_actor_id = 'exec')
      OR (kind = 'direct' AND mentioned_actor_id <> 'exec')
    ),
    CHECK (jsonb_typeof(alternatives) = 'array'),
    CHECK (jsonb_typeof(evidence) = 'array'),
    CHECK (why_this_actor IS NULL OR length(btrim(why_this_actor)) BETWEEN 1 AND 2000),
    CHECK (expected_response IS NULL OR length(btrim(expected_response)) BETWEEN 1 AND 2000),
    CHECK (recommendation IS NULL OR length(btrim(recommendation)) BETWEEN 1 AND 4000),
    CHECK (uncertainty IS NULL OR length(btrim(uncertainty)) BETWEEN 1 AND 2000),
    CHECK (affected_scope IS NULL OR length(btrim(affected_scope)) BETWEEN 1 AND 1000),
    CHECK (fallback IS NULL OR length(btrim(fallback)) BETWEEN 1 AND 2000),
    CHECK (
      independent_work_can_continue
      OR (work_id IS NOT NULL AND deadline_at IS NOT NULL AND fallback IS NOT NULL)
    ),
    CHECK (
      (claim_token IS NULL AND claimed_at IS NULL AND claimed_until IS NULL)
      OR (claim_token IS NOT NULL AND claimed_at IS NOT NULL AND claimed_until IS NOT NULL
          AND claimed_until > claimed_at)
    ),
    CHECK (
      (resolution_message_id IS NULL AND resolved_event_id IS NULL AND resolved_at IS NULL)
      OR (resolution_message_id IS NOT NULL AND resolved_event_id IS NOT NULL AND resolved_at IS NOT NULL)
    ),
    CHECK (resolution_claim_token IS NULL OR resolution_message_id IS NOT NULL),
    CHECK (
      (cancelled_event_id IS NULL AND cancelled_by IS NULL
       AND cancellation_reason IS NULL AND cancelled_at IS NULL)
      OR (cancelled_event_id IS NOT NULL AND cancelled_by IS NOT NULL
          AND cancellation_reason IS NOT NULL
          AND length(btrim(cancellation_reason)) BETWEEN 1 AND 2000
          AND cancelled_at IS NOT NULL)
    ),
    CHECK (NOT (resolution_message_id IS NOT NULL AND cancelled_event_id IS NOT NULL))
);

CREATE INDEX message_mentions_recipient_pending
  ON message_mentions (mentioned_actor_id, created_event_id)
  WHERE resolution_message_id IS NULL AND cancelled_event_id IS NULL;
CREATE INDEX message_mentions_recipient_claimable
  ON message_mentions (mentioned_actor_id, claimed_until, created_event_id)
  WHERE resolution_message_id IS NULL AND cancelled_event_id IS NULL;
CREATE INDEX message_mentions_room_message
  ON message_mentions (room_id, message_id, created_event_id);

-- A Room-creation request may be retried after its HTTP acknowledgement is
-- lost. Canonical company/direct Room keys prevent duplicate Rooms but cannot
-- detect command semantic drift, and ordinary group Rooms have no canonical
-- key. This is only a receipt over the authoritative Room, not another Room
-- store.
CREATE TABLE room_creation_commands (
    created_by            TEXT NOT NULL REFERENCES actors(id),
    client_command_id     TEXT NOT NULL,
    client_payload_sha256 TEXT NOT NULL,
    room_id               UUID NOT NULL REFERENCES rooms(id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (created_by, client_command_id),
    CHECK (length(btrim(client_command_id)) BETWEEN 1 AND 128),
    CHECK (client_payload_sha256 ~ '^[0-9a-f]{64}$')
);
CREATE INDEX room_creation_commands_room ON room_creation_commands (room_id);

-- A body-free latency hint only. The scheduler rereads the unresolved mention
-- and current Room participation after commit; a lost notification is repaired
-- by its ordinary company scan.
CREATE OR REPLACE FUNCTION orgintel_notify_message_mention() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('restless_orgintel', json_build_object(
    'company', TG_TABLE_SCHEMA,
    'kind', 'mention',
    'body', json_build_object(
      'mention_id', NEW.id,
      'room_id', NEW.room_id,
      'message_id', NEW.message_id,
      'to', NEW.mentioned_actor_id
    )
  )::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER message_mention_notify
  AFTER INSERT ON message_mentions
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_message_mention();

-- One durable Actor may own at most one sovereign conversation process across
-- daemon replicas. Productive Work remains authoritative in work_attempts;
-- both claim paths serialize on the same Actor row and exclude the other.
CREATE TABLE actor_cognitive_leases (
    actor_id             TEXT PRIMARY KEY REFERENCES actors(id),
    lease_token          UUID NOT NULL UNIQUE,
    focused_mention_id   UUID UNIQUE,
    claimed_at           TIMESTAMPTZ NOT NULL,
    claimed_until        TIMESTAMPTZ NOT NULL,
    revoked_at            TIMESTAMPTZ,
    revoked_by            TEXT REFERENCES actors(id),
    revocation_reason     TEXT,
    FOREIGN KEY (focused_mention_id, actor_id)
      REFERENCES message_mentions(id, mentioned_actor_id),
    CHECK (claimed_until > claimed_at),
    CHECK (
      (revoked_at IS NULL AND revoked_by IS NULL AND revocation_reason IS NULL)
      OR (revoked_at IS NOT NULL AND revoked_by IS NOT NULL
          AND revocation_reason IS NOT NULL
          AND length(btrim(revocation_reason)) BETWEEN 1 AND 2000)
    )
);

-- One automatic owner-facing final answer per cognitive process. The lease
-- token supplies the stable command id; its payload digest detects semantic
-- drift after a lost receipt. Room commands retain their existing separate
-- `(room_id,from_actor,client_command_id)` boundary.
ALTER TABLE messages DROP CONSTRAINT messages_client_command_shape;
ALTER TABLE messages
  ADD COLUMN conversation_focus_receipt_after_message_id BIGINT,
  ADD COLUMN conversation_focus_receipt_started_at TIMESTAMPTZ,
  ADD CONSTRAINT messages_conversation_focus_receipt_shape CHECK (
    conversation_focus_receipt_after_message_id IS NULL
    OR conversation_focus_receipt_after_message_id >= 0
  );
ALTER TABLE messages ADD CONSTRAINT messages_client_command_shape CHECK (
  (client_command_id IS NULL AND client_payload_sha256 IS NULL)
  OR (
    (room_id IS NOT NULL OR (room_id IS NULL AND to_actor IS NULL))
    AND length(btrim(client_command_id)) BETWEEN 1 AND 128
    AND client_payload_sha256 ~ '^[0-9a-f]{64}$'
  )
);

-- A Work-linked Message keeps its immutable author, original audience and
-- Direct-Room transcript. When responsibility moves, this projection records
-- the one current delivery recipient without copying the Message body into a
-- second feedback row. Work/Attempt context continues to read the one source
-- Message, while conversation delivery follows `routed_to_actor`.
ALTER TABLE work_feedback
  ADD COLUMN routed_to_actor TEXT REFERENCES actors(id),
  ADD COLUMN routed_at TIMESTAMPTZ,
  ADD COLUMN routed_by TEXT REFERENCES actors(id),
  ADD COLUMN route_reason TEXT,
  ADD CONSTRAINT work_feedback_route_shape CHECK (
    (routed_to_actor IS NULL AND routed_at IS NULL
      AND routed_by IS NULL AND route_reason IS NULL)
    OR
    (routed_to_actor IS NOT NULL AND routed_at IS NOT NULL
      AND routed_by IS NOT NULL AND route_reason IS NOT NULL
      AND length(btrim(route_reason)) BETWEEN 1 AND 2000)
  );
CREATE INDEX work_feedback_routed_recipient
  ON work_feedback (routed_to_actor, message_id)
  WHERE routed_to_actor IS NOT NULL;
-- Legacy-message compatibility assigns the deterministic owner/Actor Direct
-- Room in a BEFORE INSERT trigger. The existing Room command unique index is
-- therefore the actual concurrency fence for cognitive final replies too;
-- their unguessable lease-derived command id supplies the stable receipt key.

-- Attachment bytes live on the company computer, but ambiguous PostgreSQL
-- commit acknowledgements need one durable cross-system fence. Register each
-- daemon-generated UUID before writing bytes. The Message transaction locks
-- and links the exact command rows; bounded collection claims only old,
-- still-unlinked rows. Commit and collection therefore serialize instead of
-- relying on a time-window assumption.
CREATE TABLE owner_attachments (
    attachment_id         UUID PRIMARY KEY,
    sender_actor_id       TEXT NOT NULL REFERENCES actors(id),
    target_actor_id       TEXT NOT NULL REFERENCES actors(id),
    client_command_id     TEXT NOT NULL,
    client_payload_sha256 TEXT NOT NULL,
    canonical_name        TEXT NOT NULL,
    canonical_media_type  TEXT NOT NULL,
    size_bytes            BIGINT NOT NULL,
    content_sha256        TEXT NOT NULL,
    message_id            BIGINT REFERENCES messages(id),
    linked_at             TIMESTAMPTZ,
    staging_finished_at   TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    gc_claimed_at         TIMESTAMPTZ,
    gc_claim_token        UUID,
    purge_requested_at    TIMESTAMPTZ,
    purge_requested_by    TEXT REFERENCES actors(id),
    purge_reason          TEXT,
    purged_at             TIMESTAMPTZ,
    CHECK (length(btrim(client_command_id)) BETWEEN 1 AND 128),
    CHECK (client_payload_sha256 ~ '^[0-9a-f]{64}$'),
    CHECK (octet_length(canonical_name) BETWEEN 1 AND 1024),
    CHECK (octet_length(canonical_media_type) BETWEEN 1 AND 255),
    CHECK (size_bytes BETWEEN 0 AND 5242880),
    CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    CHECK ((message_id IS NULL) = (linked_at IS NULL)),
    CHECK (staging_finished_at IS NULL OR message_id IS NOT NULL),
    CHECK ((gc_claimed_at IS NULL) = (gc_claim_token IS NULL)),
    CHECK (
      (purge_requested_at IS NULL AND purge_requested_by IS NULL
       AND purge_reason IS NULL AND purged_at IS NULL)
      OR (purge_requested_at IS NOT NULL AND purge_requested_by IS NOT NULL
          AND purge_reason IS NOT NULL
          AND length(btrim(purge_reason)) BETWEEN 1 AND 2000)
    ),
    CHECK (purged_at IS NULL OR purge_requested_at IS NOT NULL)
);
CREATE INDEX owner_attachments_collect
  ON owner_attachments ((message_id IS NOT NULL) DESC, created_at, attachment_id)
  WHERE staging_finished_at IS NULL OR (purge_requested_at IS NOT NULL AND purged_at IS NULL);
