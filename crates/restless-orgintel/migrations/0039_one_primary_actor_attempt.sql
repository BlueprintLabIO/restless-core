-- Sprint 49: one durable Actor may own at most one sovereign productive
-- session at a time. The scheduler already intends this policy in its claim
-- query; the partial unique index makes concurrent daemon scans obey it too.
CREATE UNIQUE INDEX work_attempts_one_running_per_actor
  ON work_attempts (actor_id)
  WHERE state = 'running';
