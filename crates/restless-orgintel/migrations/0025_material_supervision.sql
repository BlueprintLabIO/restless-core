-- S30: retain accountable supervision while making the nominal producing
-- topology and its commissioner exact. Existing Work is backfilled from its
-- durable team relation; new Work always records both fields explicitly.

CREATE TYPE producing_topology AS ENUM (
  'coherent_single_worker',
  'locally_closing_parallel_unit'
);

ALTER TABLE work
  ADD COLUMN producing_topology producing_topology NOT NULL DEFAULT 'coherent_single_worker',
  ADD COLUMN commissioned_by TEXT REFERENCES actors(id);

UPDATE work commissioned
SET commissioned_by = COALESCE(
  (
    SELECT team.lead_actor_id
    FROM actors owner
    JOIN teams team ON team.id = owner.team_id
    WHERE owner.id = commissioned.owner_id
  ),
  commissioned.owner_id
);

ALTER TABLE work
  ALTER COLUMN commissioned_by SET NOT NULL;
