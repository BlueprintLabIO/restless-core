-- Complete the native-Document command boundary. Metadata and named-version
-- mutations were initially optimistic but could still repeat after a committed
-- response was lost. Every client-visible mutation now has one durable command
-- identity and one request fingerprint.

ALTER TABLE native_document_command_receipts
  DROP CONSTRAINT native_document_command_receipts_operation_check;

ALTER TABLE native_document_command_receipts
  ADD CONSTRAINT native_document_command_receipts_operation_check CHECK (operation IN (
    'document_create',
    'document_metadata_update',
    'named_version_create',
    'named_version_restore',
    'participant_set',
    'participant_remove',
    'comment_thread_create',
    'comment_reply',
    'comment_thread_resolve',
    'review_request',
    'review_accept',
    'revision_propose',
    'revision_accept',
    'revision_reject'
  ));
