CREATE TABLE schedule_recovery_retries (
  schedule_id UUID NOT NULL,
  scheduled_for TIMESTAMPTZ NOT NULL,
  retry_key TEXT NOT NULL,
  prior_message_id BIGINT NOT NULL REFERENCES messages(id),
  message_id BIGINT NOT NULL UNIQUE REFERENCES messages(id),
  retried_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  retried_by TEXT NOT NULL,
  reason TEXT NOT NULL,
  PRIMARY KEY (schedule_id, scheduled_for, retry_key),
  FOREIGN KEY (schedule_id, scheduled_for)
    REFERENCES schedule_recoveries(schedule_id, scheduled_for)
);
