-- One For you row a day for the changes Exec applied on others' direction (S80), keyed by the
-- company-local day: `direction-day:YYYY-MM-DD`. It is read and archived like any other row.
ALTER TABLE for_you_reads DROP CONSTRAINT for_you_reads_item_key_check;
ALTER TABLE for_you_reads ADD CONSTRAINT for_you_reads_item_key_check
  CHECK (
    item_key ~ '^[a-z-]{1,24}:[0-9a-f-]{36}(:[0-9]{1,19})?$'
    OR item_key ~ '^direction-day:[0-9]{4}-[0-9]{2}-[0-9]{2}$'
  );
