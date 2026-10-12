-- People own teams; teams own goals; agents do the work.
--
-- A team may have one responsible person: a human member who directs it, answers the
-- judgement its agent lead and Exec pass up, and accepts its outcomes. A team with nobody
-- falls back to the owner, which is how every company behaved before this migration.
ALTER TABLE teams ADD COLUMN responsible_actor_id TEXT REFERENCES actors(id);
CREATE INDEX teams_responsible ON teams (responsible_actor_id)
  WHERE responsible_actor_id IS NOT NULL AND disbanded_at IS NULL;

-- Every appointment and revocation, with who made it and the owner message it cites when
-- Exec recorded it from a conversation.
CREATE TABLE team_responsibility_events (
    id                 UUID PRIMARY KEY,
    team_id            UUID NOT NULL REFERENCES teams(id),
    actor_id           TEXT REFERENCES actors(id),
    previous_actor_id  TEXT REFERENCES actors(id),
    changed_by         TEXT NOT NULL REFERENCES actors(id),
    cited_message_id   BIGINT,
    reason             TEXT NOT NULL CHECK (length(reason) BETWEEN 1 AND 2000),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX team_responsibility_events_team ON team_responsibility_events (team_id, created_at);

-- A goal belongs to the team that delivers it.
ALTER TABLE goals ADD COLUMN team_id UUID REFERENCES teams(id) ON DELETE SET NULL;
CREATE INDEX goals_team ON goals (team_id) WHERE team_id IS NOT NULL;

-- Existing goals go to the live team whose staff own most of their Work. Goals without
-- team-owned Work stay without a team, which means the owner.
UPDATE goals g SET team_id = pick.team_id
FROM (
    SELECT DISTINCT ON (w.goal_id) w.goal_id, a.team_id
    FROM work w
    JOIN actors a ON a.id = w.owner_id AND a.team_id IS NOT NULL
    JOIN teams t ON t.id = a.team_id AND t.disbanded_at IS NULL
    WHERE w.goal_id IS NOT NULL
    GROUP BY w.goal_id, a.team_id
    ORDER BY w.goal_id, count(*) DESC, a.team_id
) pick
WHERE g.id = pick.goal_id AND g.team_id IS NULL;

-- When Exec last reminded the person a judgement is waiting on, so each wait gets one nudge.
ALTER TABLE owner_handoffs ADD COLUMN person_reminded_at TIMESTAMPTZ;
