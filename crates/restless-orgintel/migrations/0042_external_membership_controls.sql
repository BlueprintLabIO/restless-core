-- Hosted membership remains owned by the external identity plane. Core keeps
-- the latest accepted state so an already-issued handoff cannot recreate
-- access after suspension or removal, while preserving the durable Actor and
-- all historical attribution.
ALTER TABLE human_principal_actor_bindings
  ADD COLUMN membership_status TEXT NOT NULL DEFAULT 'active',
  ADD COLUMN last_controlled_at TIMESTAMPTZ,
  ADD CONSTRAINT human_principal_binding_status_known
    CHECK (membership_status IN ('active', 'suspended', 'removed'));

-- A terminal control can arrive before this company has ever seen the human.
-- That denial is deliberately separate from an Actor binding: no person is
-- manufactured merely because Fleet revoked an unseen membership.
CREATE TABLE external_membership_denials (
  issuer TEXT NOT NULL,
  subject TEXT NOT NULL,
  company_id UUID NOT NULL,
  cell_id UUID NOT NULL,
  membership_id TEXT NOT NULL,
  membership_role TEXT NOT NULL CHECK (membership_role IN ('owner', 'admin', 'member')),
  membership_status TEXT NOT NULL CHECK (membership_status IN ('suspended', 'removed')),
  membership_version BIGINT NOT NULL CHECK (membership_version >= 0),
  last_controlled_at TIMESTAMPTZ NOT NULL,
  first_verified_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_verified_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (issuer, membership_id, company_id)
);

-- Each outbox jti has one immutable request fingerprint and one durable
-- result. This is both the retry receipt and the defence against reusing a jti
-- with different signed coordinates after a process restart.
CREATE TABLE external_membership_control_receipts (
  issuer TEXT NOT NULL,
  jti UUID NOT NULL,
  claim_fingerprint BYTEA NOT NULL CHECK (octet_length(claim_fingerprint) = 32),
  owner_id UUID NOT NULL,
  plane_id UUID NOT NULL,
  plane_hostname TEXT NOT NULL,
  company_id UUID NOT NULL,
  cell_id UUID NOT NULL,
  principal_id TEXT NOT NULL,
  membership_id TEXT NOT NULL,
  membership_role TEXT NOT NULL CHECK (membership_role IN ('owner', 'admin', 'member')),
  requested_status TEXT NOT NULL CHECK (requested_status IN ('suspended', 'removed')),
  requested_version BIGINT NOT NULL CHECK (requested_version >= 0),
  outcome TEXT NOT NULL CHECK (outcome IN ('applied', 'already_applied', 'superseded')),
  observed_status TEXT NOT NULL CHECK (observed_status IN ('active', 'suspended', 'removed')),
  observed_version BIGINT NOT NULL CHECK (observed_version >= 0),
  observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (issuer, jti)
);

CREATE INDEX external_membership_denials_principal_idx
  ON external_membership_denials (issuer, subject, company_id);

CREATE INDEX external_membership_control_receipts_membership_idx
  ON external_membership_control_receipts
    (issuer, company_id, membership_id, requested_version);
