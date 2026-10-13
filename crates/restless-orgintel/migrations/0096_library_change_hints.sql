-- Documents, sheets and their sharing, comments and asks change live for everyone (S70). Each
-- change sends a body-free `library_changed` hint on the shared channel; open cockpits refetch
-- what is on screen, answered only from what each person may read. The scheduler ignores this
-- kind. Identical hints within one transaction reach listeners once.
--
-- Live typing rewrites a document's or sheet's row many times a minute, so those rows hint only
-- when what a list shows changes: title, status, sharing or owner. Personal marks (read state,
-- pins, follows) stay out: one person's read would otherwise refresh everyone.
CREATE OR REPLACE FUNCTION orgintel_notify_library_changed() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('restless_orgintel', json_build_object(
    'company', TG_TABLE_SCHEMA,
    'kind', 'library_changed',
    'body', json_build_object()
  )::text);
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER native_documents_library_changed_insert
  AFTER INSERT OR DELETE ON native_documents
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_documents_library_changed_update
  AFTER UPDATE ON native_documents
  FOR EACH ROW
  WHEN (OLD.title IS DISTINCT FROM NEW.title
    OR OLD.status IS DISTINCT FROM NEW.status
    OR OLD.visibility IS DISTINCT FROM NEW.visibility
    OR OLD.company_access IS DISTINCT FROM NEW.company_access
    OR OLD.owner_actor_id IS DISTINCT FROM NEW.owner_actor_id
    OR OLD.current_named_version_id IS DISTINCT FROM NEW.current_named_version_id)
  EXECUTE FUNCTION orgintel_notify_library_changed();

CREATE TRIGGER native_sheets_library_changed_insert
  AFTER INSERT OR DELETE ON native_sheets
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_sheets_library_changed_update
  AFTER UPDATE ON native_sheets
  FOR EACH ROW
  WHEN (OLD.title IS DISTINCT FROM NEW.title
    OR OLD.archived_at IS DISTINCT FROM NEW.archived_at
    OR OLD.visibility IS DISTINCT FROM NEW.visibility
    OR OLD.company_access IS DISTINCT FROM NEW.company_access
    OR OLD.owner_actor_id IS DISTINCT FROM NEW.owner_actor_id)
  EXECUTE FUNCTION orgintel_notify_library_changed();

CREATE TRIGGER native_document_participants_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_document_participants
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_sheet_participants_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_sheet_participants
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_document_comment_threads_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_document_comment_threads
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_document_comments_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_document_comments
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_document_comment_reactions_library_changed
  AFTER INSERT OR DELETE ON native_document_comment_reactions
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_document_access_requests_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_document_access_requests
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER native_sheet_access_requests_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON native_sheet_access_requests
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
CREATE TRIGGER pending_shares_library_changed
  AFTER INSERT OR UPDATE OR DELETE ON pending_shares
  FOR EACH ROW EXECUTE FUNCTION orgintel_notify_library_changed();
