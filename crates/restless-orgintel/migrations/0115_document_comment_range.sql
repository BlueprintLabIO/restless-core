-- A comment in the margin (Sprint 109 E4) points at the exact words it is about with two Yjs
-- relative positions, so its highlight follows the words through everyone's edits. Core keeps
-- them opaque (base64 of Y.encodeRelativePosition); the block and quote stay the fallback for
-- views without the live document (saved versions, agents, exports).
ALTER TABLE native_document_comment_threads
  ADD COLUMN anchor_from TEXT,
  ADD COLUMN anchor_to TEXT,
  ADD CONSTRAINT native_document_comment_threads_anchor_range CHECK (
    (anchor_from IS NULL AND anchor_to IS NULL)
    OR (
      block_id IS NOT NULL
      AND char_length(anchor_from) BETWEEN 4 AND 258
      AND char_length(anchor_to) BETWEEN 4 AND 258
      AND anchor_from ~ '^[A-Za-z0-9+/]+={0,2}$'
      AND anchor_to ~ '^[A-Za-z0-9+/]+={0,2}$'
    )
  );
