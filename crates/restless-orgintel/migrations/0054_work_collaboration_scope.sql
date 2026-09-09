-- Work becomes visible to invited human collaborators through one explicit,
-- small-company audience rule. Company Work is readable by every active
-- Actor. Room Work is readable only by active participants of one linked
-- Room. Authority and mutation rights remain separate concerns.

CREATE TYPE work_collaboration_visibility AS ENUM ('company', 'room');

ALTER TABLE work
  ADD COLUMN collaboration_visibility work_collaboration_visibility
    NOT NULL DEFAULT 'company',
  ADD COLUMN collaboration_room_id UUID REFERENCES rooms(id),
  ADD COLUMN collaboration_revision BIGINT NOT NULL DEFAULT 1
    CHECK (collaboration_revision >= 1),
  ADD CONSTRAINT work_collaboration_scope_shape CHECK (
    (collaboration_visibility='company' AND collaboration_room_id IS NULL)
    OR
    (collaboration_visibility='room' AND collaboration_room_id IS NOT NULL)
  );

CREATE INDEX work_collaboration_room_idx
  ON work (collaboration_room_id, updated_at DESC, id)
  WHERE collaboration_room_id IS NOT NULL;

-- Retrying a scope change after a lost response returns the first committed
-- result. The payload digest makes command-id reuse with different intent a
-- visible conflict rather than a silent audience change.
CREATE TABLE work_collaboration_scope_commands (
  actor_id TEXT NOT NULL REFERENCES actors(id),
  command_id UUID NOT NULL,
  work_id UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
  payload_sha256 TEXT NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
  resulting_visibility work_collaboration_visibility NOT NULL,
  resulting_room_id UUID REFERENCES rooms(id),
  resulting_collaboration_revision BIGINT NOT NULL
    CHECK (resulting_collaboration_revision >= 1),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (actor_id, command_id),
  CHECK (
    (resulting_visibility='company' AND resulting_room_id IS NULL)
    OR
    (resulting_visibility='room' AND resulting_room_id IS NOT NULL)
  ),
  UNIQUE (work_id, actor_id, command_id)
);

CREATE INDEX work_collaboration_scope_commands_work_idx
  ON work_collaboration_scope_commands (work_id, created_at DESC);
