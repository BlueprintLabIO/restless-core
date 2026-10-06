-- An app request (Sprint 63): the app that would unblock this Work, as a
-- catalogue key or the address the owner adds. Allowing that app resolves the
-- handoff as an observation, so the owner never reports that sign-in is done.
ALTER TABLE owner_handoffs
  ADD COLUMN app TEXT CHECK (app IS NULL OR (length(app) BETWEEN 1 AND 512 AND app !~ '\s'));
CREATE INDEX owner_handoffs_pending_app ON owner_handoffs (app) WHERE state = 'pending' AND app IS NOT NULL;
