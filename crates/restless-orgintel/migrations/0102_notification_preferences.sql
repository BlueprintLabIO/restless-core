-- Whether a person wants email for what needs them (Sprint 83). No row means yes: email is
-- how the Inbox reaches someone whose cockpit is closed.
CREATE TABLE notification_preferences (
  actor_id text PRIMARY KEY,
  email boolean NOT NULL DEFAULT true,
  updated_at timestamptz NOT NULL DEFAULT now()
);
