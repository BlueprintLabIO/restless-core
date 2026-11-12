-- The company reviews its own shape (owner decision, 12 November 2026): "self evolution should
-- happen as soon as there's enough new usage data in the company ie signal from the human
-- operator", and "rn it sort of just freezes itself into shitty org structures".
--
-- Every restructuring move already existed (release, retire, re-lead, disband) and none was ever
-- prompted, because nothing measured whether a shape worked. Two things were missing:
--
-- 1. Which team an Attempt ran under. `actors.team_id` is only the present, so once a member
--    moved, its history moved with it. Each Attempt now carries its team and lead at the time.
-- 2. A record of each review, so "enough new signal" is measured from the last one and the next
--    review can check whether the last change helped.
ALTER TABLE work_attempts
    ADD COLUMN team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    ADD COLUMN lead_actor_id TEXT REFERENCES actors(id);

-- Best available history for Attempts that ran before this: the actor's present team.
UPDATE work_attempts attempt
SET team_id = team.id, lead_actor_id = team.lead_actor_id
FROM actors actor
JOIN teams team ON team.id = actor.team_id
WHERE actor.id = attempt.actor_id;

CREATE INDEX work_attempts_by_team ON work_attempts (team_id, started_at);

-- A lead's own Attempts count for the team it leads, whether or not it is also listed as a member.
CREATE OR REPLACE FUNCTION orgintel_stamp_attempt_team() RETURNS trigger AS $$
BEGIN
    SELECT team.id, team.lead_actor_id INTO NEW.team_id, NEW.lead_actor_id
    FROM actors actor
    JOIN teams team ON team.disbanded_at IS NULL
        AND (team.lead_actor_id = actor.id OR team.id = actor.team_id)
    WHERE actor.id = NEW.actor_id
    ORDER BY (team.lead_actor_id = actor.id) DESC
    LIMIT 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER work_attempts_stamp_team
    BEFORE INSERT ON work_attempts
    FOR EACH ROW EXECUTE FUNCTION orgintel_stamp_attempt_team();

-- One row per review Exec completed. `signals_through` is the watermark: owner signals after it
-- are new. `scorecard` is the per-team numbers the review saw, kept so the next review can say
-- what its changes did.
CREATE TABLE org_reviews (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    reviewed_by TEXT NOT NULL REFERENCES actors(id),
    reviewed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    signals_through TIMESTAMPTZ NOT NULL,
    signal_count INTEGER NOT NULL CHECK (signal_count >= 0),
    scorecard JSONB NOT NULL,
    -- What the review concluded and changed, in the reviewer's words.
    conclusion TEXT NOT NULL CHECK (char_length(btrim(conclusion)) BETWEEN 1 AND 1200)
);
