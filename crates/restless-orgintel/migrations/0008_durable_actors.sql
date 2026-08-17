-- S06-T3. Actor identity survives assignments and revisions. Retirement is a
-- reversible OrgIntel fact, not deletion: historical Work, messages, effects
-- and artifacts keep pointing at the actor that actually did them.
ALTER TABLE actors
  ADD COLUMN retired_at        TIMESTAMPTZ,
  ADD COLUMN retired_by        TEXT REFERENCES actors(id),
  ADD COLUMN retirement_reason TEXT NOT NULL DEFAULT '';
CREATE INDEX actors_active ON actors (created_at, id) WHERE retired_at IS NULL;
