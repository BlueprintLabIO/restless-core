-- Gate retirement preserves historical declarations and runs, but the
-- original table-wide uniqueness constraint made its name impossible to
-- reuse for the corrected active declaration. Uniqueness belongs to the
-- active gate set; retired rows are immutable evidence.
ALTER TABLE work_gates
  DROP CONSTRAINT IF EXISTS work_gates_work_id_name_key;

CREATE UNIQUE INDEX IF NOT EXISTS work_gates_one_active_name
  ON work_gates (work_id, name)
  WHERE retired_at IS NULL;
