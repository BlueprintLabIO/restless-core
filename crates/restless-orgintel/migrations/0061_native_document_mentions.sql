-- A mention is a durable obligation to answer one native comment, not another mailbox.
CREATE TABLE native_document_mentions (
 id UUID PRIMARY KEY,
 document_id UUID NOT NULL,
 thread_id UUID NOT NULL,
 comment_id UUID NOT NULL,
 mentioned_actor_id TEXT NOT NULL REFERENCES actors(id),
 claim_token UUID,
 resolution_comment_id UUID,
 cancelled_at TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE (comment_id, mentioned_actor_id),
 UNIQUE (id, mentioned_actor_id),
 FOREIGN KEY (document_id, thread_id) REFERENCES native_document_comment_threads(document_id,id) ON DELETE CASCADE,
 FOREIGN KEY (thread_id, comment_id) REFERENCES native_document_comments(thread_id,id),
 FOREIGN KEY (thread_id, resolution_comment_id) REFERENCES native_document_comments(thread_id,id),
 CHECK (resolution_comment_id IS NULL OR cancelled_at IS NULL)
);
CREATE INDEX native_document_mentions_pending ON native_document_mentions(mentioned_actor_id,created_at,id)
 WHERE resolution_comment_id IS NULL AND cancelled_at IS NULL;
ALTER TABLE actor_cognitive_leases ADD COLUMN focused_document_mention_id UUID UNIQUE;
ALTER TABLE actor_cognitive_leases ADD FOREIGN KEY (focused_document_mention_id,actor_id)
 REFERENCES native_document_mentions(id,mentioned_actor_id);
ALTER TABLE actor_cognitive_leases ADD CHECK (focused_mention_id IS NULL OR focused_document_mention_id IS NULL);

-- Actor-authored continuity stays scoped to the exact document mention.
ALTER TABLE actor_context_checkpoints ADD COLUMN focused_document_mention_id UUID REFERENCES native_document_mentions(id);
ALTER TABLE actor_context_checkpoints ADD CHECK (focused_document_mention_id IS NULL OR (session_kind='cognitive_session' AND focused_mention_id IS NULL));
CREATE INDEX actor_context_checkpoints_document_mention_idx ON actor_context_checkpoints(actor_id,focused_document_mention_id,checkpoint_version DESC) WHERE focused_document_mention_id IS NOT NULL;

-- Committed separately before the admission constraint uses this enum value.
ALTER TYPE model_invocation_kind ADD VALUE 'document_mention';

-- A latency hint only; the durable row and access checks remain authoritative.
CREATE FUNCTION orgintel_notify_document_mention() RETURNS trigger AS $$
BEGIN
 PERFORM pg_notify('restless_orgintel',json_build_object(
  'company',TG_TABLE_SCHEMA,'kind','mention',
  'body',json_build_object('source_kind','document','mention_id',NEW.id,
   'document_id',NEW.document_id,'thread_id',NEW.thread_id,
   'comment_id',NEW.comment_id,'to',NEW.mentioned_actor_id)
 )::text);
 RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER native_document_mention_notify AFTER INSERT ON native_document_mentions
 FOR EACH ROW EXECUTE FUNCTION orgintel_notify_document_mention();
