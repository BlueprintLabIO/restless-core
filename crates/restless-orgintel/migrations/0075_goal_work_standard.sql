-- The quality bar belongs to the outcome being pursued, not to the company or
-- to a team: a Goal sets it, and each piece of Work inherits its Goal's bar
-- unless it states its own.
ALTER TABLE goals
  ADD COLUMN outcome_standard outcome_standard NOT NULL DEFAULT 'exceptional';
ALTER TABLE work
  ADD COLUMN outcome_standard outcome_standard;

-- Carry the standard each Goal's Work was already commissioned under: the
-- most common standard among the teams producing that Goal's Work.
UPDATE goals goal
SET outcome_standard = carried.outcome_standard
FROM (
  SELECT DISTINCT ON (ranked.goal_id) ranked.goal_id, ranked.outcome_standard
  FROM (
    SELECT work.goal_id, team.outcome_standard, count(*) AS uses
    FROM work
    JOIN actors owner ON owner.id = work.owner_id
    JOIN teams team ON team.id = owner.team_id
    WHERE work.goal_id IS NOT NULL
    GROUP BY work.goal_id, team.outcome_standard
  ) ranked
  ORDER BY ranked.goal_id, ranked.uses DESC
) carried
WHERE goal.id = carried.goal_id;
