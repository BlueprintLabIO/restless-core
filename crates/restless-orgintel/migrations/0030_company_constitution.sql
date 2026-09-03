-- S35: executable Company Constitution integration. Existing artifact, Work,
-- Authority and effect lifecycles remain authoritative.
-- `bound_by` was proved during S32 after migration 0027 had already shipped;
-- add it forward rather than rewriting applied history.
ALTER TABLE company_voice_work_contracts
  ADD COLUMN bound_by TEXT NOT NULL DEFAULT 'legacy accountable binder';
ALTER TABLE company_voice_work_contracts
  ADD CONSTRAINT company_voice_work_contracts_bound_by_nonempty
  CHECK (btrim(bound_by) <> '');
ALTER TABLE company_voice_work_contracts ALTER COLUMN bound_by DROP DEFAULT;

CREATE TYPE company_constitution_learning_trigger AS ENUM ('owner_evidence','customer_evidence','exercised_outcome');
CREATE TYPE company_identity_drift_kind AS ENUM ('truth_stale','voice_difference','visual_difference','culture_difference','unknown_dependency');
CREATE TYPE company_identity_migration_disposition AS ENUM ('retain','revise','retire');

CREATE TABLE company_constitution_artifact_bindings (
  artifact_ref_id UUID PRIMARY KEY REFERENCES artifact_refs(id),
  work_id UUID NOT NULL REFERENCES work(id),
  release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  channel TEXT NOT NULL,
  audience TEXT NOT NULL,
  named_author TEXT NOT NULL,
  producer TEXT NOT NULL,
  accountable_lead TEXT NOT NULL,
  company_voice TEXT NOT NULL,
  native_evidence JSONB NOT NULL,
  constitution_digest TEXT NOT NULL,
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (btrim(channel)<>''),CHECK(btrim(audience)<>''),CHECK(btrim(named_author)<>''),
  CHECK (btrim(producer)<>''),CHECK(btrim(accountable_lead)<>''),CHECK(btrim(company_voice)<>''),
  CHECK (jsonb_typeof(native_evidence)='object'),CHECK(constitution_digest ~ '^[0-9a-f]{64}$')
);
CREATE TABLE company_constitution_artifact_evidence (
  artifact_ref_id UUID NOT NULL REFERENCES company_constitution_artifact_bindings(artifact_ref_id),
  evidence_id UUID NOT NULL REFERENCES company_identity_evidence(id),
  PRIMARY KEY(artifact_ref_id,evidence_id)
);

CREATE TABLE company_constitution_learning_proposals (
  proposal_id UUID PRIMARY KEY REFERENCES company_identity_proposals(id) ON DELETE CASCADE,
  evidence_id UUID NOT NULL REFERENCES company_identity_evidence(id),
  pillar company_identity_pillar NOT NULL,
  trigger_kind company_constitution_learning_trigger NOT NULL,
  triggering_event TEXT NOT NULL,
  before_artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  after_artifact_ref_id UUID NOT NULL REFERENCES artifact_refs(id),
  scope TEXT NOT NULL,
  contradiction_check TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK(before_artifact_ref_id<>after_artifact_ref_id),CHECK(btrim(triggering_event)<>''),
  CHECK(btrim(scope)<>''),CHECK(btrim(contradiction_check)<>'')
);

CREATE TABLE company_identity_drift_findings (
  id UUID PRIMARY KEY,
  artifact_ref_id UUID NOT NULL REFERENCES company_constitution_artifact_bindings(artifact_ref_id),
  from_release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  to_release_id UUID NOT NULL REFERENCES company_identity_releases(id),
  kind company_identity_drift_kind NOT NULL,
  old_evidence_id UUID REFERENCES company_identity_evidence(id),
  new_evidence_id UUID REFERENCES company_identity_evidence(id),
  dependency TEXT NOT NULL,
  consequence TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(artifact_ref_id,to_release_id,kind,old_evidence_id),
  CHECK(from_release_id<>to_release_id),CHECK(btrim(dependency)<>''),CHECK(btrim(consequence)<>'')
);
CREATE TABLE company_identity_migration_decisions (
  drift_finding_id UUID PRIMARY KEY REFERENCES company_identity_drift_findings(id),
  disposition company_identity_migration_disposition NOT NULL,
  decided_by TEXT NOT NULL,
  rationale TEXT NOT NULL,
  authority_record_id TEXT NOT NULL,
  decided_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK(btrim(decided_by)<>''),CHECK(btrim(rationale)<>''),CHECK(btrim(authority_record_id)<>'')
);
CREATE INDEX company_constitution_binding_release ON company_constitution_artifact_bindings(release_id,bound_at);
CREATE INDEX company_identity_drift_release ON company_identity_drift_findings(to_release_id,created_at);
