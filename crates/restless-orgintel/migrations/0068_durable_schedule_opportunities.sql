-- Durable scheduled responsibilities own outcomes across replaceable worker turns.
CREATE TABLE responsibilities (
  id UUID PRIMARY KEY,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  current_version INTEGER NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE responsibility_versions (
  responsibility_id UUID NOT NULL REFERENCES responsibilities(id),
  version INTEGER NOT NULL CHECK (version > 0),
  objective TEXT NOT NULL CHECK (length(btrim(objective)) > 0),
  policy JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (responsibility_id, version)
);
ALTER TABLE responsibilities
  ADD CONSTRAINT responsibility_current_version_fk
  FOREIGN KEY (id, current_version)
  REFERENCES responsibility_versions(responsibility_id, version)
  DEFERRABLE INITIALLY DEFERRED;

-- Version rows are immutable: editing intent or authority creates a new version.
CREATE FUNCTION reject_responsibility_version_mutation() RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'responsibility versions are immutable';
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER responsibility_versions_immutable
  BEFORE UPDATE OR DELETE ON responsibility_versions
  FOR EACH ROW EXECUTE FUNCTION reject_responsibility_version_mutation();

CREATE TABLE opportunities (
  id UUID PRIMARY KEY,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  responsibility_id UUID NOT NULL,
  responsibility_version INTEGER NOT NULL,
  state TEXT NOT NULL DEFAULT 'queued' CHECK (state IN (
    'queued','inspecting','executing','verifying','recovering','waiting_retry',
    'completed','needs_human','blocked','cancelled'
  )),
  outcome JSONB,
  outcome_reason TEXT,
  evidence_refs JSONB NOT NULL DEFAULT '[]',
  revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
  owner_epoch BIGINT NOT NULL DEFAULT 0 CHECK (owner_epoch >= 0),
  lease_owner TEXT,
  lease_expires_at TIMESTAMPTZ,
  next_wake_at TIMESTAMPTZ,
  deadline_at TIMESTAMPTZ,
  last_progress_at TIMESTAMPTZ,
  wake_message_id BIGINT REFERENCES messages(id),
  wake_count INTEGER NOT NULL DEFAULT 0 CHECK (wake_count >= 0),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  settled_at TIMESTAMPTZ,
  FOREIGN KEY (responsibility_id, responsibility_version)
    REFERENCES responsibility_versions(responsibility_id, version),
  CHECK ((lease_owner IS NULL) = (lease_expires_at IS NULL)),
  CHECK (jsonb_typeof(evidence_refs) = 'array'),
  CHECK ((state IN ('completed','needs_human','blocked','cancelled')) = (settled_at IS NOT NULL))
);
CREATE UNIQUE INDEX one_open_opportunity_per_responsibility
  ON opportunities (responsibility_id)
  WHERE state NOT IN ('completed','needs_human','blocked','cancelled');
CREATE INDEX opportunities_due ON opportunities (next_wake_at)
  WHERE state IN ('queued','waiting_retry');

CREATE TABLE opportunity_wakes (
  opportunity_id UUID NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK (sequence > 0),
  message_id BIGINT NOT NULL UNIQUE REFERENCES messages(id),
  reason TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (opportunity_id, sequence)
);

ALTER TABLE schedules
  ADD COLUMN responsibility_id UUID,
  ADD COLUMN responsibility_version INTEGER,
  ADD CONSTRAINT schedule_responsibility_binding_shape CHECK (
    (responsibility_id IS NULL AND responsibility_version IS NULL)
    OR (responsibility_id IS NOT NULL AND responsibility_version IS NOT NULL)
  ),
  ADD CONSTRAINT schedule_responsibility_version_fk
    FOREIGN KEY (responsibility_id, responsibility_version)
    REFERENCES responsibility_versions(responsibility_id, version);

ALTER TABLE schedule_occurrences
  ADD COLUMN opportunity_id UUID REFERENCES opportunities(id),
  ADD COLUMN responsibility_id UUID,
  ADD COLUMN responsibility_version INTEGER,
  ADD COLUMN admission TEXT CHECK (admission IN ('admitted','coalesced','skipped')),
  ADD COLUMN wake_message_id BIGINT REFERENCES messages(id),
  ADD CONSTRAINT occurrence_responsibility_binding_shape CHECK (
    (responsibility_id IS NULL AND responsibility_version IS NULL)
    OR (responsibility_id IS NOT NULL AND responsibility_version IS NOT NULL)
  ),
  ADD CONSTRAINT occurrence_responsibility_version_fk
    FOREIGN KEY (responsibility_id, responsibility_version)
    REFERENCES responsibility_versions(responsibility_id, version);
CREATE INDEX schedule_occurrences_opportunity ON schedule_occurrences (opportunity_id)
  WHERE opportunity_id IS NOT NULL;

CREATE TABLE opportunity_work (
  opportunity_id UUID NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
  work_id UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
  linked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  relation TEXT NOT NULL DEFAULT 'primary' CHECK (relation IN ('primary','supporting')),
  PRIMARY KEY (opportunity_id, work_id)
);
CREATE UNIQUE INDEX one_primary_work_per_opportunity
  ON opportunity_work (opportunity_id) WHERE relation='primary';
