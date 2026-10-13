-- Finding a sheet by what is in it (S78). Each sheet keeps the text of its cells as last saved:
-- one line per cell, "Tab!A1 content". Formulas are left out: their text is code, not words.
-- The workbook is the engine's export, as every accepted change already carries it.
CREATE OR REPLACE FUNCTION native_sheet_text(workbook jsonb) RETURNS text
LANGUAGE sql IMMUTABLE AS $$
  SELECT left(coalesce(string_agg(cell.line, E'\n' ORDER BY cell.tab, cell.name), ''), 1000000)
  FROM (
    SELECT tab.ordinality AS tab, entry.key AS name,
      (tab.value->>'name') || '!' || entry.key || ' ' || text.content AS line,
      text.content
    FROM jsonb_array_elements(coalesce(workbook->'sheets', '[]'::jsonb)) WITH ORDINALITY AS tab(value, ordinality)
    CROSS JOIN LATERAL jsonb_each(coalesce(tab.value->'cells', '{}'::jsonb)) AS entry(key, value)
    CROSS JOIN LATERAL (
      SELECT CASE jsonb_typeof(entry.value)
        WHEN 'object' THEN coalesce(entry.value->>'content', '')
        ELSE coalesce(entry.value #>> '{}', '')
      END AS content
    ) AS text
  ) AS cell
  WHERE cell.content <> '' AND cell.content NOT LIKE '=%'
$$;

ALTER TABLE native_sheets ADD COLUMN search_text TEXT NOT NULL DEFAULT '';

-- Existing sheets start from their newest checkpoint, or the workbook they began as; the next
-- change to each brings it up to date.
UPDATE native_sheets s SET search_text = native_sheet_text(coalesce(
  (SELECT c.workbook FROM native_sheet_checkpoints c WHERE c.sheet_id = s.id
    ORDER BY c.sequence DESC, c.created_at DESC LIMIT 1),
  s.replay_base));
