-- S33: durable visual language. Visual rules remain evidence inside the
-- immutable Company Identity release; native captures and art-direction
-- decisions bind exact artifacts, viewports and motion states.

CREATE TYPE company_visual_evidence_kind AS ENUM (
  'semantic_token', 'typography_role', 'composition_principle',
  'imagery_direction', 'motion_pattern', 'product_representation_rule',
  'primitive', 'approved_composition', 'rejected_example'
);
CREATE TYPE company_visual_channel AS ENUM ('landing_page', 'email', 'product', 'social');
CREATE TYPE company_visual_representation AS ENUM ('exact_product', 'clearly_abstract', 'none');
CREATE TYPE company_visual_motion_state AS ENUM ('full', 'reduced', 'static');
CREATE TYPE company_visual_review_verdict AS ENUM ('accept', 'revise', 'reject');

CREATE TABLE company_visual_evidence_details (
  evidence_id UUID PRIMARY KEY REFERENCES company_identity_evidence(id) ON DELETE CASCADE,
  kind company_visual_evidence_kind NOT NULL,
  channel company_visual_channel,
  purpose TEXT NOT NULL,
  rationale TEXT NOT NULL,
  semantic_role TEXT,
  value TEXT,
  reduced_motion_replacement TEXT,
  product_truth_locator TEXT,
  origin TEXT,
  licence TEXT,
  framework TEXT,
  dependencies JSONB NOT NULL DEFAULT '[]'::jsonb,
  adaptation_status TEXT,
  accessibility_notes TEXT NOT NULL,
  CHECK (btrim(purpose) <> ''),
  CHECK (btrim(rationale) <> ''),
  CHECK (btrim(accessibility_notes) <> ''),
  CHECK (jsonb_typeof(dependencies) = 'array'),
  CHECK (kind <> 'motion_pattern' OR btrim(COALESCE(reduced_motion_replacement, '')) <> ''),
  CHECK (kind <> 'product_representation_rule' OR btrim(COALESCE(product_truth_locator, '')) <> ''),
  CHECK (kind <> 'primitive' OR (
    btrim(COALESCE(origin, '')) <> '' AND btrim(COALESCE(licence, '')) <> ''
    AND btrim(COALESCE(framework, '')) <> '' AND btrim(COALESCE(adaptation_status, '')) <> ''
  )),
  CHECK (kind <> 'semantic_token' OR (
    btrim(COALESCE(semantic_role, '')) <> '' AND btrim(COALESCE(value, '')) <> ''
  ))
);

CREATE TABLE company_visual_work_contracts (
  work_id UUID PRIMARY KEY REFERENCES work(id) ON DELETE CASCADE,
  release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  channel company_visual_channel NOT NULL,
  bound_by TEXT NOT NULL,
  audience TEXT NOT NULL,
  outcome TEXT NOT NULL,
  information_hierarchy TEXT NOT NULL,
  proof TEXT NOT NULL,
  density TEXT NOT NULL,
  imagery_role TEXT NOT NULL,
  motion_role TEXT NOT NULL,
  product_representation company_visual_representation NOT NULL,
  product_truth_locator TEXT,
  requested_departure TEXT,
  contract_digest TEXT NOT NULL,
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(bound_by) <> ''), CHECK (btrim(audience) <> ''), CHECK (btrim(outcome) <> ''),
  CHECK (btrim(information_hierarchy) <> ''), CHECK (btrim(proof) <> ''), CHECK (btrim(density) <> ''),
  CHECK (btrim(imagery_role) <> ''), CHECK (btrim(motion_role) <> ''),
  CHECK (product_representation <> 'exact_product' OR btrim(COALESCE(product_truth_locator, '')) <> ''),
  CHECK (contract_digest ~ '^[0-9a-f]{64}$')
);

CREATE TABLE company_visual_primitive_uses (
  work_id UUID NOT NULL REFERENCES company_visual_work_contracts(work_id) ON DELETE CASCADE,
  evidence_id UUID NOT NULL REFERENCES company_visual_evidence_details(evidence_id),
  primitive_version TEXT NOT NULL,
  purpose TEXT NOT NULL,
  PRIMARY KEY (work_id, evidence_id),
  CHECK (btrim(primitive_version) <> ''), CHECK (btrim(purpose) <> '')
);

CREATE TABLE company_visual_render_evidence (
  id UUID PRIMARY KEY,
  work_id UUID NOT NULL REFERENCES company_visual_work_contracts(work_id),
  artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  channel company_visual_channel NOT NULL,
  renderer TEXT NOT NULL,
  renderer_version TEXT NOT NULL,
  viewport_width INTEGER NOT NULL,
  viewport_height INTEGER NOT NULL,
  motion_state company_visual_motion_state NOT NULL,
  native_checks JSONB NOT NULL,
  captured_by TEXT NOT NULL,
  captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(renderer) <> ''), CHECK (btrim(renderer_version) <> ''),
  CHECK (viewport_width > 0 AND viewport_height > 0),
  CHECK (jsonb_typeof(native_checks) = 'object'), CHECK (btrim(captured_by) <> '')
);

CREATE TABLE company_visual_reviews (
  id UUID PRIMARY KEY,
  render_evidence_id UUID NOT NULL REFERENCES company_visual_render_evidence(id),
  control_render_evidence_id UUID REFERENCES company_visual_render_evidence(id),
  reviewer TEXT NOT NULL,
  verdict company_visual_review_verdict NOT NULL,
  identity_findings TEXT NOT NULL DEFAULT '',
  hierarchy_findings TEXT NOT NULL DEFAULT '',
  density_findings TEXT NOT NULL DEFAULT '',
  proof_findings TEXT NOT NULL DEFAULT '',
  product_fidelity_findings TEXT NOT NULL DEFAULT '',
  motion_findings TEXT NOT NULL DEFAULT '',
  defect_findings TEXT NOT NULL DEFAULT '',
  departure_decision TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(reviewer) <> ''),
  CHECK (verdict = 'accept' OR btrim(identity_findings || hierarchy_findings || density_findings ||
    proof_findings || product_fidelity_findings || motion_findings || defect_findings) <> '')
);

CREATE INDEX company_visual_evidence_channel ON company_visual_evidence_details(channel, kind, evidence_id);
CREATE INDEX company_visual_render_artifact ON company_visual_render_evidence(artifact_ref_id, captured_at DESC);
CREATE INDEX company_visual_review_render ON company_visual_reviews(render_evidence_id, created_at DESC);
