-- Sprint 45: authentication principal, durable company Actor identity, and
-- organisational role are separate facts. `kind` remains as a compatibility
-- projection while callers migrate; `actor_class` is the canonical class.
ALTER TABLE actors ADD COLUMN actor_class TEXT;

UPDATE actors
SET actor_class = CASE kind
  WHEN 'owner' THEN 'human'
  WHEN 'exec' THEN 'agent'
  WHEN 'staff' THEN 'agent'
  WHEN 'system' THEN 'service'
  ELSE 'service'
END;

ALTER TABLE actors ALTER COLUMN actor_class SET NOT NULL;
ALTER TABLE actors ADD CONSTRAINT actors_actor_class_check
  CHECK (actor_class IN ('human', 'agent', 'service'));
ALTER TABLE actors DROP CONSTRAINT actors_kind_known;
ALTER TABLE actors ADD CONSTRAINT actors_kind_known
  CHECK (kind IN ('owner', 'exec', 'staff', 'system', 'human'));

-- One OrgIntel schema is one company. Hosted bootstrap binds the immutable
-- Fleet company/cell coordinates once; later handoffs must match exactly.
CREATE TABLE company_access_identity (
  singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
  company_id UUID NOT NULL UNIQUE,
  cell_id UUID NOT NULL UNIQUE,
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The IdP principal remains private authentication identity. Company history
-- names the durable Actor. A fresh membership for the same issuer/subject
-- updates this row without manufacturing a new colleague.
CREATE TABLE human_principal_actor_bindings (
  issuer TEXT NOT NULL,
  subject TEXT NOT NULL,
  company_id UUID NOT NULL,
  actor_id TEXT NOT NULL UNIQUE REFERENCES actors(id),
  membership_id TEXT NOT NULL,
  membership_role TEXT NOT NULL CHECK (membership_role IN ('owner', 'admin', 'member')),
  membership_version BIGINT NOT NULL CHECK (membership_version >= 0),
  last_asserted_at TIMESTAMPTZ NOT NULL,
  first_verified_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_verified_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (issuer, subject, company_id),
  UNIQUE (issuer, membership_id, company_id)
);

-- Handoffs are credentials and are single use across process restarts. Rows
-- may be compacted after expiry, but uniqueness is durable while they matter.
CREATE TABLE consumed_entry_assertions (
  issuer TEXT NOT NULL,
  jti UUID NOT NULL,
  company_id UUID NOT NULL,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  expires_at TIMESTAMPTZ NOT NULL,
  consumed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (issuer, jti)
);

CREATE INDEX consumed_entry_assertions_expiry_idx
  ON consumed_entry_assertions (expires_at);

CREATE INDEX human_principal_actor_membership_idx
  ON human_principal_actor_bindings (actor_id, membership_id, membership_version);
