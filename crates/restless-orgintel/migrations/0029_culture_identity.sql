-- S34: observable operating culture. No employee scores, sentiment,
-- personality or disciplinary state exists in this schema.
CREATE TYPE company_culture_evidence_kind AS ENUM (
  'founding_decision', 'observed_conduct', 'counterexample', 'promoted_norm', 'bounded_exception'
);
CREATE TYPE company_culture_case AS ENUM (
  'disagreement', 'uncertain_incident', 'customer_recovery', 'quality_tradeoff', 'hiring'
);
CREATE TYPE company_culture_confidence AS ENUM ('tentative', 'corroborated', 'owner_founded');
CREATE TYPE company_culture_review_verdict AS ENUM ('accept', 'revise', 'reject');

CREATE TABLE company_culture_evidence_details (
  evidence_id UUID PRIMARY KEY REFERENCES company_identity_evidence(id) ON DELETE CASCADE,
  kind company_culture_evidence_kind NOT NULL,
  case_kind company_culture_case,
  situation TEXT NOT NULL,
  consequence TEXT NOT NULL,
  actors TEXT NOT NULL,
  decision_authority TEXT NOT NULL,
  conduct TEXT NOT NULL,
  observed_outcome TEXT NOT NULL,
  confidence company_culture_confidence NOT NULL,
  counterexample TEXT NOT NULL,
  boundary_conditions TEXT NOT NULL,
  operational_implication TEXT NOT NULL,
  actor_scope TEXT NOT NULL,
  CHECK (btrim(situation) <> ''), CHECK (btrim(consequence) <> ''), CHECK (btrim(actors) <> ''),
  CHECK (btrim(decision_authority) <> ''), CHECK (btrim(conduct) <> ''),
  CHECK (btrim(observed_outcome) <> ''), CHECK (btrim(counterexample) <> ''),
  CHECK (btrim(boundary_conditions) <> ''), CHECK (btrim(operational_implication) <> ''),
  CHECK (btrim(actor_scope) <> '')
);

CREATE TABLE company_culture_work_contracts (
  work_id UUID PRIMARY KEY REFERENCES work(id) ON DELETE CASCADE,
  release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  case_kind company_culture_case NOT NULL,
  actor TEXT NOT NULL,
  actor_role TEXT NOT NULL,
  team TEXT NOT NULL,
  consequence TEXT NOT NULL,
  decision_boundary TEXT NOT NULL,
  bound_by TEXT NOT NULL,
  contract_digest TEXT NOT NULL,
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(actor) <> ''), CHECK (btrim(actor_role) <> ''), CHECK (btrim(team) <> ''),
  CHECK (btrim(consequence) <> ''), CHECK (btrim(decision_boundary) <> ''), CHECK (btrim(bound_by) <> ''),
  CHECK (contract_digest ~ '^[0-9a-f]{64}$')
);

CREATE TABLE company_culture_case_records (
  id UUID PRIMARY KEY,
  work_id UUID NOT NULL REFERENCES company_culture_work_contracts(work_id),
  artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  case_kind company_culture_case NOT NULL,
  decision TEXT NOT NULL,
  alternatives JSONB NOT NULL,
  unknowns TEXT NOT NULL,
  correction_of UUID REFERENCES company_culture_case_records(id),
  correction_account TEXT NOT NULL DEFAULT '',
  customer_action TEXT NOT NULL DEFAULT '',
  native_checks JSONB NOT NULL,
  recorded_by TEXT NOT NULL,
  recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(decision) <> ''), CHECK (jsonb_typeof(alternatives) = 'array'),
  CHECK (btrim(unknowns) <> ''), CHECK (jsonb_typeof(native_checks) = 'object'),
  CHECK (btrim(recorded_by) <> ''),
  CHECK ((correction_of IS NULL AND correction_account = '') OR
         (correction_of IS NOT NULL AND btrim(correction_account) <> '')),
  CHECK (case_kind <> 'customer_recovery' OR btrim(customer_action) <> '')
);

CREATE TABLE company_culture_reviews (
  id UUID PRIMARY KEY,
  case_record_id UUID NOT NULL REFERENCES company_culture_case_records(id),
  reviewer TEXT NOT NULL,
  verdict company_culture_review_verdict NOT NULL,
  conduct_findings TEXT NOT NULL DEFAULT '',
  dissent_findings TEXT NOT NULL DEFAULT '',
  uncertainty_findings TEXT NOT NULL DEFAULT '',
  correction_findings TEXT NOT NULL DEFAULT '',
  authority_findings TEXT NOT NULL DEFAULT '',
  customer_or_hiring_findings TEXT NOT NULL DEFAULT '',
  slogan_recitation_detected BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(reviewer) <> ''),
  CHECK (verdict = 'accept' OR btrim(conduct_findings || dissent_findings || uncertainty_findings ||
    correction_findings || authority_findings || customer_or_hiring_findings) <> '')
);

CREATE INDEX company_culture_evidence_case ON company_culture_evidence_details(case_kind, actor_scope, evidence_id);
CREATE INDEX company_culture_case_work ON company_culture_case_records(work_id, recorded_at DESC);
CREATE INDEX company_culture_review_case ON company_culture_reviews(case_record_id, created_at DESC);
