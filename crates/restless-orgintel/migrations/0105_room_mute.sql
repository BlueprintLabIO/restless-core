-- A participant may mute a conversation (Sprint 83): its ordinary messages stop counting as
-- unread and stop notifying them. A mention still reaches them.
ALTER TABLE room_participants ADD COLUMN muted boolean NOT NULL DEFAULT false;
