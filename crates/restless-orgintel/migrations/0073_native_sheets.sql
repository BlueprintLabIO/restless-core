-- Core-native workbook state is company-local OrgIntel, never Authority.
CREATE TABLE native_sheets (
  id uuid PRIMARY KEY,
  title text NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
  owner_actor_id text NOT NULL REFERENCES actors(id),
  visibility text NOT NULL CHECK (visibility IN ('company','participants')),
  engine_version text NOT NULL,
  replay_base jsonb NOT NULL,
  head_revision text NOT NULL,
  sequence bigint NOT NULL DEFAULT 0,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE native_sheet_participants (
  sheet_id uuid NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  actor_id text NOT NULL REFERENCES actors(id),
  access text NOT NULL CHECK (access IN ('read','edit')),
  PRIMARY KEY(sheet_id,actor_id)
);
CREATE TABLE native_sheet_messages (
  sheet_id uuid NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  sequence bigint NOT NULL,
  revision_id text NOT NULL,
  actor_id text NOT NULL REFERENCES actors(id),
  message jsonb NOT NULL,
  accepted_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY(sheet_id,sequence),
  UNIQUE(sheet_id,revision_id)
);
CREATE TABLE native_sheet_checkpoints (
  id uuid PRIMARY KEY,
  sheet_id uuid NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  sequence bigint NOT NULL,
  revision_id text NOT NULL,
  workbook jsonb NOT NULL,
  title text,
  actor_id text NOT NULL REFERENCES actors(id),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX native_sheet_checkpoints_history ON native_sheet_checkpoints(sheet_id,sequence DESC);
CREATE TABLE native_sheet_commands (
  sheet_id uuid NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  command_id uuid NOT NULL,
  actor_id text NOT NULL REFERENCES actors(id),
  request jsonb NOT NULL,
  result jsonb NOT NULL,
  PRIMARY KEY(sheet_id,command_id)
);
CREATE TABLE native_sheet_clients (
  sheet_id uuid NOT NULL REFERENCES native_sheets(id) ON DELETE CASCADE,
  client_id uuid NOT NULL,
  actor_id text NOT NULL REFERENCES actors(id),
  reconnect_hash bytea NOT NULL,
  generation uuid NOT NULL,
  PRIMARY KEY(sheet_id,client_id)
);
