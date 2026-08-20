-- S08-T8/T9 · Actor class and organisational role are different facts.
--
-- `kind` was documented as exec | staff | owner | system in 0001, but the
-- implementation stored each actor's craft in it. Preserve that craft as
-- `role`, then restore the small actor-class vocabulary used by projections.
ALTER TABLE actors RENAME COLUMN kind TO role;
ALTER TABLE actors ADD COLUMN kind TEXT;

UPDATE actors
SET kind = CASE
    WHEN id = 'owner' THEN 'owner'
    WHEN id = 'exec' THEN 'exec'
    WHEN id IN ('world', 'daemon') THEN 'system'
    ELSE 'staff'
END;

-- Repair the two provenance-only senders without changing their stable ids or
-- deleting any message attribution.
UPDATE actors SET role = 'external-sender' WHERE id = 'world';
UPDATE actors SET role = 'system-sender' WHERE id = 'daemon';

ALTER TABLE actors ALTER COLUMN kind SET NOT NULL;
ALTER TABLE actors ADD CONSTRAINT actors_kind_known
    CHECK (kind IN ('owner', 'exec', 'staff', 'system'));
