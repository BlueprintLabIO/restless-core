-- Library pins follow a person to every device (S65). Pinning stays a per-person convenience,
-- never company state: one row per person and pinned item, and the key names the source object
-- (`doc:<id>`, `sheet:<id>`, a Work file or a kept file), never a copy. Whether the person may
-- still open what a pin names is decided where the item is read, so a pin grants nothing.
CREATE TABLE library_pins (
  actor_id TEXT NOT NULL REFERENCES actors(id),
  item_key TEXT NOT NULL CHECK (item_key ~ '^[a-z]{1,16}:[A-Za-z0-9._:-]{1,200}$'),
  pinned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (actor_id, item_key)
);
