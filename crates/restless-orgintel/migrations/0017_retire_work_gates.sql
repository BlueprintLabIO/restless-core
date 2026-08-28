-- A malformed deterministic gate used to be permanent: leads could append a
-- corrected gate, but every later Attempt still ran the bad one and therefore
-- could never close. Gates are recoverable coordination state, so preserve
-- their historical runs while allowing the active declaration to be retired.
ALTER TABLE work_gates ADD COLUMN retired_at TIMESTAMPTZ;
ALTER TABLE work_gates ADD COLUMN retired_by TEXT;
ALTER TABLE work_gates ADD COLUMN retired_reason TEXT;

ALTER TABLE work_gates ADD CONSTRAINT work_gates_retirement_complete CHECK (
    (retired_at IS NULL AND retired_by IS NULL AND retired_reason IS NULL)
    OR
    (retired_at IS NOT NULL AND retired_by IS NOT NULL AND retired_reason IS NOT NULL)
);
