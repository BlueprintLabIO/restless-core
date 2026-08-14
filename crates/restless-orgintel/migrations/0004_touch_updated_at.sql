-- updated_at is bookkeeping, not content: the row is ground truth, so the
-- database maintains it for every writer (OrgIntel method, psql, tooling).
-- The T6 reconcile sweep depends on it being trustworthy.

CREATE OR REPLACE FUNCTION orgintel_touch_updated_at() RETURNS trigger AS $$
BEGIN
  NEW.updated_at := now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER commitments_touch_updated_at
  BEFORE UPDATE ON commitments
  FOR EACH ROW EXECUTE FUNCTION orgintel_touch_updated_at();
