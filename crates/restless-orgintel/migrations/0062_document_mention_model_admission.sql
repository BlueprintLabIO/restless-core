ALTER TABLE model_invocation_admissions ADD COLUMN document_mention_id UUID REFERENCES native_document_mentions(id);
DO $$
DECLARE c RECORD; removed INTEGER := 0;
BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='model_invocation_admissions'::regclass AND contype='c' AND pg_get_constraintdef(oid) LIKE '%subject_id%'
 LOOP
  EXECUTE format('ALTER TABLE model_invocation_admissions DROP CONSTRAINT %I',c.conname);
  removed := removed + 1;
 END LOOP;
 IF removed <> 1 THEN RAISE EXCEPTION 'expected one model invocation source constraint, found %',removed; END IF;
END $$;
ALTER TABLE model_invocation_admissions ADD CONSTRAINT model_invocation_source_coordinates CHECK (
 (kind='work' AND work_id IS NOT NULL AND attempt_id IS NOT NULL AND cognitive_lease_token IS NULL AND mention_id IS NULL AND document_mention_id IS NULL AND subject_id=attempt_id)
 OR (kind='owner_conversation' AND work_id IS NULL AND attempt_id IS NULL AND cognitive_lease_token IS NOT NULL AND mention_id IS NULL AND document_mention_id IS NULL AND subject_id=cognitive_lease_token)
 OR (kind='room_mention' AND work_id IS NULL AND attempt_id IS NULL AND cognitive_lease_token IS NOT NULL AND mention_id IS NOT NULL AND document_mention_id IS NULL AND subject_id=cognitive_lease_token)
 OR (kind='document_mention' AND work_id IS NULL AND attempt_id IS NULL AND cognitive_lease_token IS NOT NULL AND mention_id IS NULL AND document_mention_id IS NOT NULL AND subject_id=cognitive_lease_token)
);
