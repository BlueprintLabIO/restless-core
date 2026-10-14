-- How the company organises its Library (Sprint 83): a collection label on any item, and for
-- a file the owner kept, a display name. Company state, shared by everyone who can see the item;
-- the key names the source object as pins do, and grants nothing. A Work file's name lives on
-- its artifact reference instead, so every view of it shows the same name.
CREATE TABLE library_items (
  item_key TEXT PRIMARY KEY CHECK (item_key ~ '^[a-z]{1,16}:[A-Za-z0-9._:-]{1,200}$'),
  collection TEXT CHECK (collection IS NULL OR char_length(collection) BETWEEN 1 AND 80),
  label TEXT CHECK (label IS NULL OR char_length(label) BETWEEN 1 AND 200),
  updated_by TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX library_items_collection ON library_items (collection) WHERE collection IS NOT NULL;

-- An archived Work file the owner removed from the Library for good. The reference stays, as
-- the provenance other records point at; it just no longer appears anywhere in the Library.
ALTER TABLE artifact_refs ADD COLUMN library_removed_at TIMESTAMPTZ;
