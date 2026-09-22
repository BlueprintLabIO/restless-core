-- Preserve the immutable owner target of each collaboration request. Existing
-- requests predate network-entered owner identities and were addressed to the
-- bootstrap owner Actor.
ALTER TABLE native_document_collaboration_requests
    ADD COLUMN requested_owner_actor_id TEXT REFERENCES actors(id);

UPDATE native_document_collaboration_requests
SET requested_owner_actor_id = 'owner'
WHERE requested_owner_actor_id IS NULL;

ALTER TABLE native_document_collaboration_requests
    ALTER COLUMN requested_owner_actor_id SET NOT NULL;
