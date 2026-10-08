-- Inbox items the owner set aside as not needed. An item stays hidden until it is raised again
-- after the dismissal. Recoverable owner-surface state, not governance: dismissing grants or
-- declines nothing.
CREATE TABLE attention_dismissals (
  item_id TEXT PRIMARY KEY CHECK (length(item_id) BETWEEN 1 AND 400),
  dismissed_by TEXT NOT NULL CHECK (length(dismissed_by) BETWEEN 1 AND 200),
  dismissed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
