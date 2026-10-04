-- A reaction is the cheapest reply: "noted", "looking", "done". It is stored
-- with the message it answers and never wakes an agent on its own; the agent
-- sees it the next time it reads the conversation.
CREATE TABLE message_reactions (
  message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  emoji TEXT NOT NULL CHECK (length(emoji) BETWEEN 1 AND 16),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (message_id, actor_id, emoji)
);
CREATE INDEX message_reactions_message ON message_reactions (message_id);
