-- Whom a person asked to review a document (Sprint 83). NULL keeps the earlier meaning: any editor
-- may judge it, and it reaches the company's owner. A Work-bound review keeps its reviewer on the
-- Work dependency.
ALTER TABLE native_document_reviews ADD COLUMN reviewer_actor_id TEXT REFERENCES actors(id);
