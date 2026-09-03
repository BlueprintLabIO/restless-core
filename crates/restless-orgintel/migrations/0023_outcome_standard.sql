-- S29: one durable ambition contract follows an owner request into the
-- commissioned team. This remains coordination and company policy, never an
-- authority grant or a model-spend allocation.
CREATE TYPE outcome_standard AS ENUM ('fast', 'thorough', 'exceptional', 'frontier');
CREATE TYPE outcome_standard_source AS ENUM ('company_default', 'owner_override', 'owner_language');

ALTER TABLE messages
  ADD COLUMN outcome_standard outcome_standard;

ALTER TABLE teams
  ADD COLUMN outcome_standard outcome_standard NOT NULL DEFAULT 'exceptional',
  ADD COLUMN outcome_standard_source outcome_standard_source NOT NULL DEFAULT 'company_default',
  ADD COLUMN standard_source_message_id BIGINT REFERENCES messages(id);

ALTER TABLE teams ADD CONSTRAINT team_standard_source_is_grounded CHECK (
  (outcome_standard_source = 'company_default' AND standard_source_message_id IS NULL)
  OR
  (outcome_standard_source IN ('owner_override', 'owner_language') AND standard_source_message_id IS NOT NULL)
);
