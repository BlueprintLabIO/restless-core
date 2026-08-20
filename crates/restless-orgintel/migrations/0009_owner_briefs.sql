-- S07-T1/T2. Owner meaning is prepared once by the accountable actor and
-- stored with the existing organisational handoff. Operational truth remains
-- in Work/Attempts/artifacts; this compact payload explains one exact source
-- snapshot and is invalidated by a changed fingerprint.

ALTER TABLE owner_handoffs
  ADD COLUMN owner_brief JSONB,
  ADD COLUMN briefed_by TEXT REFERENCES actors(id),
  ADD COLUMN briefed_at TIMESTAMPTZ,
  ADD COLUMN brief_source_fingerprint TEXT,
  ADD CONSTRAINT owner_brief_is_object
    CHECK (owner_brief IS NULL OR jsonb_typeof(owner_brief) = 'object'),
  ADD CONSTRAINT owner_brief_attribution_complete
    CHECK (
      (owner_brief IS NULL AND briefed_by IS NULL AND briefed_at IS NULL
        AND brief_source_fingerprint IS NULL)
      OR
      (owner_brief IS NOT NULL AND briefed_by IS NOT NULL AND briefed_at IS NOT NULL
        AND brief_source_fingerprint IS NOT NULL)
    );

-- A pre-S07 ordinary judgement has never passed the new attention-admission
-- check. Return it to Exec for resolution or preparation rather than dumping
-- its internal prose into the owner's primary surface. The five irreducible
-- human categories remain direct.
UPDATE owner_handoffs
SET assigned_to = 'exec',
    escalated_from = COALESCE(escalated_from, requested_by),
    escalated_at = COALESCE(escalated_at, now()),
    resolution = CASE
      WHEN resolution = '' THEN 'owner brief required before attention admission'
      ELSE resolution
    END
WHERE state = 'pending'
  AND category = 'owner_judgement'
  AND assigned_to IS NULL
  AND owner_brief IS NULL;

