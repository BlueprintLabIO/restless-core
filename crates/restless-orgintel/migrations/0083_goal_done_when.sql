-- A Goal is an outcome the owner would celebrate: what "done" looks like, observably, and by when.
ALTER TABLE goals
  ADD COLUMN done_when TEXT NOT NULL DEFAULT '' CHECK (length(done_when) <= 400),
  ADD COLUMN due_on DATE;
