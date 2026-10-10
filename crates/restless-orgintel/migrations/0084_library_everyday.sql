-- Everyday Library work (S64): files the owner adds, files the owner archives, and documents
-- the owner deletes for good.

-- A file the owner adds to the Library without a message is kept by that decision rather than by
-- a Message reference. It counts against the same retention limits and is purged the same way.
ALTER TABLE owner_attachments ADD COLUMN kept_at TIMESTAMPTZ;
ALTER TABLE owner_attachments ADD CONSTRAINT owner_attachments_kept_without_message
  CHECK (kept_at IS NULL OR message_id IS NULL);

-- Who took a Work file out of the Library, so the owner can put back what the owner archived
-- without reviving a version Work itself replaced.
ALTER TABLE artifact_refs ADD COLUMN retired_by TEXT;

-- A named version, comment, review or receipt stays immutable while its document exists. The
-- owner may delete an archived document as a whole: that one transaction names the document it
-- purges, and only rows of that document may then be deleted.
CREATE FUNCTION orgintel_native_document_purge_only()
RETURNS trigger AS $$
BEGIN
  IF current_setting('restless.native_document_purge', true) = OLD.document_id::text THEN
    RETURN OLD;
  END IF;
  RAISE EXCEPTION 'native document history is deleted only with its whole document';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER native_document_versions_are_immutable ON native_document_versions;
CREATE TRIGGER native_document_versions_are_immutable
  BEFORE UPDATE ON native_document_versions
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_version_mutation();
CREATE TRIGGER native_document_versions_purge_only
  BEFORE DELETE ON native_document_versions
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_runtime_imports_are_immutable ON native_document_runtime_imports;
CREATE TRIGGER native_document_runtime_imports_are_immutable
  BEFORE UPDATE ON native_document_runtime_imports
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_runtime_import_mutation();
CREATE TRIGGER native_document_runtime_imports_purge_only
  BEFORE DELETE ON native_document_runtime_imports
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_comments_are_immutable ON native_document_comments;
CREATE TRIGGER native_document_comments_are_immutable
  BEFORE UPDATE ON native_document_comments
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_comment_mutation();
CREATE TRIGGER native_document_comments_purge_only
  BEFORE DELETE ON native_document_comments
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_comment_thread_evidence_is_immutable
  ON native_document_comment_threads;
CREATE TRIGGER native_document_comment_thread_evidence_is_immutable
  BEFORE UPDATE ON native_document_comment_threads
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_comment_thread_mutation();
CREATE TRIGGER native_document_comment_threads_purge_only
  BEFORE DELETE ON native_document_comment_threads
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_command_receipts_are_immutable ON native_document_command_receipts;
CREATE TRIGGER native_document_command_receipts_are_immutable
  BEFORE UPDATE ON native_document_command_receipts
  FOR EACH ROW EXECUTE FUNCTION orgintel_reject_native_document_command_receipt_mutation();
CREATE TRIGGER native_document_command_receipts_purge_only
  BEFORE DELETE ON native_document_command_receipts
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_review_request_is_immutable ON native_document_reviews;
CREATE TRIGGER native_document_review_request_is_immutable
  BEFORE UPDATE ON native_document_reviews
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_review_mutation();
CREATE TRIGGER native_document_reviews_purge_only
  BEFORE DELETE ON native_document_reviews
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_revision_proposal_is_immutable ON native_document_revision_proposals;
CREATE TRIGGER native_document_revision_proposal_is_immutable
  BEFORE UPDATE ON native_document_revision_proposals
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_revision_proposal_mutation();
CREATE TRIGGER native_document_revision_proposals_purge_only
  BEFORE DELETE ON native_document_revision_proposals
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();

DROP TRIGGER native_document_work_review_target_is_immutable
  ON native_document_work_review_dependencies;
CREATE TRIGGER native_document_work_review_target_is_immutable
  BEFORE UPDATE ON native_document_work_review_dependencies
  FOR EACH ROW EXECUTE FUNCTION orgintel_guard_native_document_work_review_mutation();
CREATE TRIGGER native_document_work_review_dependencies_purge_only
  BEFORE DELETE ON native_document_work_review_dependencies
  FOR EACH ROW EXECUTE FUNCTION orgintel_native_document_purge_only();
