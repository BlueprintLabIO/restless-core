-- Acceptance commands form a declared pipeline. Preserve that declaration
-- order instead of deriving execution order from timestamps or random UUIDs.
ALTER TABLE work_gates ADD COLUMN sequence_no INTEGER;

WITH ordered AS (
    SELECT id,
           row_number() OVER (PARTITION BY work_id ORDER BY created_at, id) - 1 AS sequence_no
    FROM work_gates
)
UPDATE work_gates AS gate
SET sequence_no = ordered.sequence_no
FROM ordered
WHERE gate.id = ordered.id;

ALTER TABLE work_gates ALTER COLUMN sequence_no SET NOT NULL;
ALTER TABLE work_gates ADD CONSTRAINT work_gates_work_sequence_unique
    UNIQUE (work_id, sequence_no);
