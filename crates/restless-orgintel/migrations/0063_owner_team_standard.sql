-- An owner can adjust an existing team's quality target directly in People.
-- Direct settings are attributed atomically in team_standard_changed events;
-- a commissioning chat message remains optional provenance, never fabricated.
ALTER TABLE teams DROP CONSTRAINT team_standard_source_is_grounded;
ALTER TABLE teams ADD CONSTRAINT team_standard_source_is_grounded CHECK (
  (outcome_standard_source = 'company_default' AND standard_source_message_id IS NULL)
  OR outcome_standard_source = 'owner_override'
  OR (outcome_standard_source = 'owner_language' AND standard_source_message_id IS NOT NULL)
);
