-- Rough progress estimates on Work (owner decision, 3 November 2026): "some kind of 10x way to
-- see the % progress of tasks and work? just rough estimates given by the agents are fine", and
-- "the agent that created the work item can estimate overall % progress at each item checkpoint
-- and that can be the source of truth".
--
-- Each row is one estimate, oldest first, so the newest row is the current one and the rest are
-- its short history. Only the Work's creator (`work.commissioned_by`) or Exec records one; the
-- daemon records 100 when the Work completes. A blocked estimate has no number: it says why.
CREATE TABLE work_progress (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    work_id UUID NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('estimate','blocked','done')),
    percent SMALLINT CHECK (percent IS NULL OR percent BETWEEN 0 AND 100),
    -- What is left, in one line; for a blocked estimate, why it is blocked.
    next TEXT NOT NULL DEFAULT '' CHECK (char_length(next) <= 160 AND position(E'\n' IN next) = 0),
    estimated_by TEXT NOT NULL REFERENCES actors(id),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    -- The compact chat line this estimate posted, when it moved far enough to say so.
    posted_message_id BIGINT REFERENCES messages(id) ON DELETE SET NULL,
    CHECK ((state = 'estimate') = (percent IS NOT NULL AND percent < 100)),
    CHECK (state <> 'done' OR percent = 100),
    CHECK (state <> 'blocked' OR (percent IS NULL AND char_length(btrim(next)) > 0)),
    CHECK (state <> 'estimate' OR char_length(btrim(next)) > 0)
);
CREATE INDEX work_progress_by_work ON work_progress (work_id, id DESC);
CREATE INDEX work_progress_posted ON work_progress (posted_message_id)
    WHERE posted_message_id IS NOT NULL;

-- Views refresh when an estimate lands, as they do when the Work itself changes.
CREATE OR REPLACE FUNCTION orgintel_notify_work_progress() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('restless_orgintel', json_build_object(
    'company', TG_TABLE_SCHEMA,
    'kind', 'work_progress',
    'body', json_build_object('work_id', NEW.work_id)
  )::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER work_progress_notify
  AFTER INSERT ON work_progress
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_work_progress();
