-- A refreshed prompt replaces the same request, never a second owner request.
DROP INDEX one_pending_handoff_per_work;
CREATE UNIQUE INDEX one_pending_handoff_per_work
  ON owner_handoffs (work_id) WHERE state IN ('pending', 'preparing');
