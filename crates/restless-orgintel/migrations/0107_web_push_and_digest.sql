-- A daily time at which a person's email arrives as one batch, in their own time zone (Sprint 83).
-- No time means email as things happen.
ALTER TABLE notification_preferences
  ADD COLUMN digest_at time,
  ADD COLUMN digest_time_zone text;

-- Browsers that asked to be told, with the cockpit closed, about what needs this person. One
-- browser belongs to whoever subscribed it last. Only the endpoint and its encryption keys live
-- here; the key that signs every push stays on the owner plane.
CREATE TABLE push_subscriptions (
  endpoint text PRIMARY KEY,
  actor_id text NOT NULL,
  p256dh text NOT NULL,
  auth text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- What each browser was already pushed, claimed before sending so a pass never sends it twice.
CREATE TABLE push_deliveries (
  endpoint text NOT NULL REFERENCES push_subscriptions(endpoint) ON DELETE CASCADE,
  intent_id text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (endpoint, intent_id)
);
