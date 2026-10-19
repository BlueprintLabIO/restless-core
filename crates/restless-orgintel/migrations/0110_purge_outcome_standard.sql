-- S103: the owner removed the outcome-ambition setting. Its columns, check
-- constraint and enum types go; nothing reads them any more.
ALTER TABLE teams DROP CONSTRAINT IF EXISTS team_standard_source_is_grounded;
ALTER TABLE teams
  DROP COLUMN IF EXISTS outcome_standard,
  DROP COLUMN IF EXISTS outcome_standard_source,
  DROP COLUMN IF EXISTS standard_source_message_id;
ALTER TABLE goals DROP COLUMN IF EXISTS outcome_standard;
ALTER TABLE work DROP COLUMN IF EXISTS outcome_standard;
ALTER TABLE messages DROP COLUMN IF EXISTS outcome_standard;
DROP TYPE IF EXISTS outcome_standard_source;
DROP TYPE IF EXISTS outcome_standard;
