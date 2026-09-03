CREATE TABLE schedule_recoveries (
  schedule_id UUID NOT NULL,
  scheduled_for TIMESTAMPTZ NOT NULL,
  message_id BIGINT NOT NULL UNIQUE REFERENCES messages(id),
  recovered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  recovered_by TEXT NOT NULL,
  reason TEXT NOT NULL,
  PRIMARY KEY (schedule_id, scheduled_for),
  FOREIGN KEY (schedule_id, scheduled_for)
    REFERENCES schedule_occurrences(schedule_id, scheduled_for)
);
