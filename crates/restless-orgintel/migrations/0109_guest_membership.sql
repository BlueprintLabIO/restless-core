-- UXG-070: a guest is someone outside the company given one document or sheet. The issuer signs
-- them in with the `guest` membership role; Core admits them only to the items shared with them by
-- name, and what the company shares with everyone ("Company can view…") never includes them.
ALTER TABLE human_principal_actor_bindings
  DROP CONSTRAINT IF EXISTS human_principal_actor_bindings_membership_role_check;
ALTER TABLE human_principal_actor_bindings
  ADD CONSTRAINT human_principal_actor_bindings_membership_role_check
  CHECK (membership_role IN ('owner', 'admin', 'member', 'guest'));

ALTER TABLE external_membership_denials
  DROP CONSTRAINT IF EXISTS external_membership_denials_membership_role_check;
ALTER TABLE external_membership_denials
  ADD CONSTRAINT external_membership_denials_membership_role_check
  CHECK (membership_role IN ('owner', 'admin', 'member', 'guest'));

ALTER TABLE external_membership_control_receipts
  DROP CONSTRAINT IF EXISTS external_membership_control_receipts_membership_role_check;
ALTER TABLE external_membership_control_receipts
  ADD CONSTRAINT external_membership_control_receipts_membership_role_check
  CHECK (membership_role IN ('owner', 'admin', 'member', 'guest'));

-- Access checks ask "is this Actor a guest?" on every company-wide read.
CREATE INDEX IF NOT EXISTS human_principal_actor_guest_idx
  ON human_principal_actor_bindings (actor_id) WHERE membership_role = 'guest';
