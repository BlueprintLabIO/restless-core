-- A set-aside item keeps what the owner saw and who asked, so the Inbox can list it for restoring
-- and the asking agent learns of it in its own context instead of from an owner message.
ALTER TABLE attention_dismissals
  ADD COLUMN title TEXT NOT NULL DEFAULT '' CHECK (length(title) <= 400),
  ADD COLUMN responsible_actor TEXT CHECK (responsible_actor IS NULL OR length(responsible_actor) BETWEEN 1 AND 200);
