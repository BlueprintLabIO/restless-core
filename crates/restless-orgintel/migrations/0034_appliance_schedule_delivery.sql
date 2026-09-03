-- Sprint 38: native wake signals are hints; durable OrgIntel state decides
-- which bounded occurrence, if any, is useful after sleep or daemon downtime.
ALTER TABLE schedules
  DROP CONSTRAINT schedules_missed_policy_shape,
  ADD CONSTRAINT schedules_missed_policy_shape CHECK (
    missed_policy IN ('skip', 'skip_if_late', 'catch_up_once', 'coalesce_latest')
    AND (catch_up_grace_seconds IS NULL OR catch_up_grace_seconds > 0)
    AND (
      (missed_policy = 'skip' AND catch_up_grace_seconds IS NULL)
      OR
      (missed_policy IN ('skip_if_late', 'catch_up_once', 'coalesce_latest')
        AND catch_up_grace_seconds IS NOT NULL)
    )
  ),
  ADD COLUMN last_considered_at TIMESTAMPTZ,
  ADD COLUMN machine_requirement TEXT NOT NULL DEFAULT 'local_mac'
    CHECK (machine_requirement IN ('local_mac', 'always_on'));

-- One coalesced row covers a closed range of superseded weekday instants. This
-- keeps long downtime bounded without allowing an occurrence range to vanish.
ALTER TABLE schedule_occurrences
  ADD COLUMN supersedes_through TIMESTAMPTZ,
  ADD COLUMN superseded_count BIGINT NOT NULL DEFAULT 0 CHECK (superseded_count >= 0),
  ADD CONSTRAINT schedule_occurrence_supersession_shape CHECK (
    (superseded_count = 0 AND supersedes_through IS NULL)
    OR
    (superseded_count > 0 AND disposition = 'skipped' AND supersedes_through >= scheduled_for)
  );

-- A schedule write wakes the in-process next-due timer. The payload contains
-- identity only; the listener always rereads the canonical schedule row.
CREATE OR REPLACE FUNCTION schedule_changed_notify() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('restless_orgintel', json_build_object(
    'company', TG_TABLE_SCHEMA,
    'kind', 'schedule_changed',
    'body', json_build_object('schedule_id', NEW.id)
  )::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER schedule_changed_notify
  AFTER INSERT OR UPDATE OF fire_at, cancelled_at, missed_policy,
    catch_up_grace_seconds, machine_requirement ON schedules
  FOR EACH ROW EXECUTE FUNCTION schedule_changed_notify();
