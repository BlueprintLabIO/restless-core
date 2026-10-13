-- A role change reaches Core at once (S79): the issuer delivers it as an active membership control
-- with the new role and version, and Core keeps its receipt like any other. Denials stay terminal
-- only: an active control for someone who never entered records nothing.
ALTER TABLE external_membership_control_receipts
  DROP CONSTRAINT IF EXISTS external_membership_control_receipts_requested_status_check;
ALTER TABLE external_membership_control_receipts
  ADD CONSTRAINT external_membership_control_receipts_requested_status_check
  CHECK (requested_status IN ('active', 'suspended', 'removed'));
