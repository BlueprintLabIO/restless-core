-- S31: source-owned company expression identity. Draft evidence and proposals
-- remain recoverable OrgIntel state; only an explicit owner promotion makes a
-- release effective. Releases are immutable and Work binds to the exact
-- release it used.
CREATE TYPE company_identity_pillar AS ENUM ('truth', 'voice', 'visual', 'culture');
CREATE TYPE company_identity_statement_kind AS ENUM (
  'fact', 'belief', 'guidance', 'observation', 'example', 'exception'
);
CREATE TYPE company_identity_evidence_status AS ENUM ('active', 'disputed', 'corrected');
CREATE TYPE company_identity_polarity AS ENUM ('neutral', 'positive', 'negative');
CREATE TYPE company_identity_proposal_state AS ENUM ('pending', 'promoted', 'rejected');

CREATE TABLE company_identity_evidence (
  id UUID PRIMARY KEY,
  pillar company_identity_pillar NOT NULL,
  statement_kind company_identity_statement_kind NOT NULL,
  claim_key TEXT NOT NULL,
  statement TEXT NOT NULL,
  author_id TEXT NOT NULL REFERENCES actors(id),
  source TEXT NOT NULL,
  authority TEXT NOT NULL,
  scope TEXT NOT NULL,
  observed_at TIMESTAMPTZ NOT NULL,
  evidence_locator TEXT NOT NULL,
  polarity company_identity_polarity NOT NULL DEFAULT 'neutral',
  status company_identity_evidence_status NOT NULL DEFAULT 'active',
  channel TEXT,
  audience TEXT,
  supersedes_evidence_id UUID REFERENCES company_identity_evidence(id),
  exception_expires_at TIMESTAMPTZ,
  exception_indefinite BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(claim_key) <> ''),
  CHECK (btrim(statement) <> ''),
  CHECK (btrim(source) <> ''),
  CHECK (btrim(authority) <> ''),
  CHECK (btrim(scope) <> ''),
  CHECK (btrim(evidence_locator) <> ''),
  CHECK (
    (statement_kind = 'exception' AND (exception_expires_at IS NOT NULL OR exception_indefinite))
    OR
    (statement_kind <> 'exception' AND exception_expires_at IS NULL AND NOT exception_indefinite)
  )
);

CREATE TABLE company_identity_proposals (
  id UUID PRIMARY KEY,
  created_by TEXT NOT NULL REFERENCES actors(id),
  rationale TEXT NOT NULL,
  expected_predecessor UUID,
  state company_identity_proposal_state NOT NULL DEFAULT 'pending',
  decided_by TEXT REFERENCES actors(id),
  authority_record_id TEXT,
  decision_rationale TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  decided_at TIMESTAMPTZ,
  CHECK (btrim(rationale) <> ''),
  CHECK (
    (state = 'pending' AND decided_by IS NULL AND authority_record_id IS NULL AND decided_at IS NULL)
    OR
    (state <> 'pending' AND decided_by IS NOT NULL AND authority_record_id IS NOT NULL AND decided_at IS NOT NULL)
  )
);

CREATE TABLE company_identity_proposal_evidence (
  proposal_id UUID NOT NULL REFERENCES company_identity_proposals(id) ON DELETE CASCADE,
  evidence_id UUID NOT NULL REFERENCES company_identity_evidence(id),
  PRIMARY KEY (proposal_id, evidence_id)
);

CREATE TABLE company_identity_releases (
  id UUID PRIMARY KEY,
  predecessor UUID REFERENCES company_identity_releases(id),
  effective_from TIMESTAMPTZ NOT NULL,
  promoted_by TEXT NOT NULL REFERENCES actors(id),
  authority_record_id TEXT NOT NULL,
  change_account TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (predecessor IS NULL OR predecessor <> id),
  CHECK (btrim(authority_record_id) <> ''),
  CHECK (btrim(change_account) <> '')
);

ALTER TABLE company_identity_proposals
  ADD CONSTRAINT company_identity_proposal_predecessor_fk
  FOREIGN KEY (expected_predecessor) REFERENCES company_identity_releases(id);

CREATE TABLE company_identity_release_evidence (
  release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  evidence_id UUID NOT NULL REFERENCES company_identity_evidence(id),
  PRIMARY KEY (release_id, evidence_id)
);

CREATE TABLE company_identity_current_release (
  singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
  release_id UUID NOT NULL UNIQUE REFERENCES company_identity_releases(id)
);

CREATE TABLE company_identity_work_bindings (
  work_id UUID PRIMARY KEY REFERENCES work(id),
  release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  stale_at TIMESTAMPTZ,
  stale_reason TEXT NOT NULL DEFAULT '',
  CHECK ((stale_at IS NULL AND stale_reason = '') OR (stale_at IS NOT NULL AND btrim(stale_reason) <> ''))
);

CREATE FUNCTION company_identity_reject_release_mutation() RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'company identity releases are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER company_identity_release_immutable
BEFORE UPDATE OR DELETE ON company_identity_releases
FOR EACH ROW EXECUTE FUNCTION company_identity_reject_release_mutation();

CREATE TRIGGER company_identity_release_evidence_immutable
BEFORE UPDATE OR DELETE ON company_identity_release_evidence
FOR EACH ROW EXECUTE FUNCTION company_identity_reject_release_mutation();
