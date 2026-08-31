-- A blocked cognitive Attempt may still leave a clean, committed candidate.
-- Preserve its exact terminal Git coordinate so an explicit coordinator
-- `resume` continues from useful work instead of either discarding it or
-- trusting a mutable retained worktree.

ALTER TABLE work_attempts
  ADD COLUMN terminal_source_commit TEXT,
  ADD COLUMN terminal_source_tree TEXT,
  ADD COLUMN terminal_status_digest TEXT,
  ADD COLUMN terminal_dirty_entries INTEGER
    CHECK (terminal_dirty_entries IS NULL OR terminal_dirty_entries >= 0),
  ADD COLUMN terminal_observed_at TIMESTAMPTZ;

CREATE INDEX work_attempts_clean_blocked_candidate
  ON work_attempts (work_id, revision, attempt_no DESC)
  WHERE state = 'blocked'
    AND terminal_dirty_entries = 0
    AND terminal_source_commit IS NOT NULL
    AND terminal_source_tree IS NOT NULL;
