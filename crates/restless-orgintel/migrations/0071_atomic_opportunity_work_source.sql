-- Ordinary post-creation links remain source-free. Only Work commissioned in
-- the same transaction as its primary Opportunity link records the exact
-- schedule occurrence that was already admitted when Work became claimable.
ALTER TABLE opportunity_work
  ADD COLUMN source_schedule_id UUID,
  ADD COLUMN source_scheduled_for TIMESTAMPTZ,
  ADD CONSTRAINT opportunity_work_source_pair CHECK (
    (source_schedule_id IS NULL) = (source_scheduled_for IS NULL)
  ),
  ADD CONSTRAINT opportunity_work_source_occurrence_fk
    FOREIGN KEY (source_schedule_id, source_scheduled_for)
    REFERENCES schedule_occurrences(schedule_id, scheduled_for);
