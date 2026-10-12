-- Reactions on document comments (S65): the same small signal people leave on a chat message,
-- left on a comment instead. A reaction is stored with the comment it answers, never sends a
-- message and never wakes an agent; it goes when the comment's document is deleted.
CREATE TABLE native_document_comment_reactions (
  comment_id UUID NOT NULL REFERENCES native_document_comments(id) ON DELETE CASCADE,
  document_id UUID NOT NULL REFERENCES native_documents(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL REFERENCES actors(id),
  emoji TEXT NOT NULL CHECK (length(emoji) BETWEEN 1 AND 16),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (comment_id, actor_id, emoji)
);
CREATE INDEX native_document_comment_reactions_document
  ON native_document_comment_reactions (document_id);
