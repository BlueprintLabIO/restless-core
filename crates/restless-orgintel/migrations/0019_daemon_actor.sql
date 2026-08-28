-- A free-standing Schedule can atomically create a durable daemon-addressed
-- message before any Attempt recovery path has had reason to bootstrap this
-- system sender. Seed the internal identity with every company schema rather
-- than making schedule claiming perform a write on every idle poll.
INSERT INTO actors (id, kind, role, display)
VALUES ('daemon', 'system', 'system-sender', 'The daemon')
ON CONFLICT (id) DO NOTHING;
