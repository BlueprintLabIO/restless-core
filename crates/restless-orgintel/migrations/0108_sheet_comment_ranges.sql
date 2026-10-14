-- A sheet comment thread may cover a range (B3:D3) as well as one cell, and its address now
-- follows rows and columns inserted or deleted around it (sheet_comments::follow_structure).
ALTER TABLE native_sheet_comment_threads DROP CONSTRAINT native_sheet_comment_threads_cell_check;
ALTER TABLE native_sheet_comment_threads ADD CONSTRAINT native_sheet_comment_threads_cell_check
  CHECK (cell ~ '^[A-Z]{1,3}[1-9][0-9]{0,6}(:[A-Z]{1,3}[1-9][0-9]{0,6})?$');
