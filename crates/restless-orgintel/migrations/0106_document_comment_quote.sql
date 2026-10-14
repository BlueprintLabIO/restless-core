-- A comment may point at the words it is about (Sprint 83, UXG-227): the passage selected when the
-- thread began, quoted, so the page can mark just those words. The block stays the anchor; the
-- quote narrows it, and a quote no longer found in the block falls back to the whole block.
ALTER TABLE native_document_comment_threads
  ADD COLUMN anchor_quote TEXT
  CHECK (
    anchor_quote IS NULL
    OR (block_id IS NOT NULL AND char_length(anchor_quote) BETWEEN 1 AND 500)
  );
