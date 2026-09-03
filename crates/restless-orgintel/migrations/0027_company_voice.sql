-- S32: human company voice remains evidence inside the released Company
-- Identity, while channel contracts and copy review bind exact Work/artifacts.

CREATE TYPE company_voice_evidence_kind AS ENUM (
  'approved_passage',
  'rejected_passage',
  'expression_principle',
  'vocabulary',
  'named_author',
  'channel_observation'
);

CREATE TYPE company_voice_channel AS ENUM (
  'newsletter',
  'founder_email',
  'support',
  'transactional_email',
  'product_ui',
  'blog'
);

CREATE TYPE company_voice_review_verdict AS ENUM ('accept', 'revise', 'reject');

CREATE TYPE company_voice_learning_kind AS ENUM (
  'typo',
  'fact_correction',
  'voice_observation'
);

CREATE TABLE company_voice_evidence_details (
  evidence_id UUID PRIMARY KEY REFERENCES company_identity_evidence(id) ON DELETE CASCADE,
  kind company_voice_evidence_kind NOT NULL,
  judgement_reason TEXT NOT NULL,
  named_author TEXT,
  channel company_voice_channel,
  audience TEXT,
  CHECK (btrim(judgement_reason) <> ''),
  CHECK (named_author IS NULL OR btrim(named_author) <> ''),
  CHECK (audience IS NULL OR btrim(audience) <> '')
);

CREATE TABLE company_voice_work_contracts (
  work_id UUID PRIMARY KEY REFERENCES work(id) ON DELETE CASCADE,
  release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  channel company_voice_channel NOT NULL,
  author TEXT NOT NULL,
  audience TEXT NOT NULL,
  reader_situation TEXT NOT NULL,
  desired_understanding TEXT NOT NULL,
  desired_action TEXT NOT NULL,
  proof TEXT NOT NULL,
  consequence TEXT NOT NULL,
  contract_digest TEXT NOT NULL,
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(author) <> ''),
  CHECK (btrim(audience) <> ''),
  CHECK (btrim(reader_situation) <> ''),
  CHECK (btrim(desired_understanding) <> ''),
  CHECK (btrim(desired_action) <> ''),
  CHECK (btrim(proof) <> ''),
  CHECK (btrim(consequence) <> ''),
  CHECK (contract_digest ~ '^[0-9a-f]{64}$')
);

CREATE TABLE company_voice_render_evidence (
  id UUID PRIMARY KEY,
  artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  channel company_voice_channel NOT NULL,
  renderer TEXT NOT NULL,
  renderer_version TEXT NOT NULL,
  semantic_checks JSONB NOT NULL,
  captured_by TEXT NOT NULL,
  captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(renderer) <> ''),
  CHECK (btrim(renderer_version) <> ''),
  CHECK (jsonb_typeof(semantic_checks) = 'object'),
  CHECK (btrim(captured_by) <> '')
);

CREATE TABLE company_voice_reviews (
  id UUID PRIMARY KEY,
  render_evidence_id UUID NOT NULL REFERENCES company_voice_render_evidence(id),
  reviewer TEXT NOT NULL,
  verdict company_voice_review_verdict NOT NULL,
  factual_findings TEXT NOT NULL DEFAULT '',
  abstraction_findings TEXT NOT NULL DEFAULT '',
  repetition_findings TEXT NOT NULL DEFAULT '',
  channel_findings TEXT NOT NULL DEFAULT '',
  authorship_findings TEXT NOT NULL DEFAULT '',
  concepts_removed TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(reviewer) <> ''),
  CHECK (
    verdict = 'accept' OR
    btrim(factual_findings || abstraction_findings || repetition_findings || channel_findings || authorship_findings) <> ''
  )
);

CREATE TABLE company_voice_learning_proposals (
  proposal_id UUID PRIMARY KEY REFERENCES company_identity_proposals(id) ON DELETE CASCADE,
  evidence_id UUID NOT NULL REFERENCES company_identity_evidence(id),
  before_artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  after_artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  change_kind company_voice_learning_kind NOT NULL,
  motivating_decision TEXT NOT NULL,
  scope TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (before_artifact_ref_id <> after_artifact_ref_id),
  CHECK (change_kind = 'voice_observation'),
  CHECK (btrim(motivating_decision) <> ''),
  CHECK (btrim(scope) <> '')
);

CREATE INDEX company_voice_evidence_channel
  ON company_voice_evidence_details (channel, kind, evidence_id);
CREATE INDEX company_voice_render_artifact
  ON company_voice_render_evidence (artifact_ref_id, captured_at DESC);
CREATE INDEX company_voice_review_render
  ON company_voice_reviews (render_evidence_id, created_at DESC);
